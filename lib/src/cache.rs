/* Copyright 2024-2025 Marco Köpcke
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use async_rwlock::{RwLock, RwLockReadGuard, RwLockUpgradableReadGuard};
use std::ops::{Add, Deref};
use std::time::{Duration, Instant};

/// A type that caches the internal type for a specific time and recreates it if the cache
/// is expired or otherwise invalidated.
pub struct Cached<T>
where
    T: LoadCacheObject,
{
    value: RwLock<Option<(Instant, T)>>,
    params: T::Params,
    valid_for: Duration,
}

impl<T, P> Cached<T>
where
    T: LoadCacheObject<Params = P>,
{
    pub fn new(params: P) -> Self {
        Self::new_expire_after(params, Duration::from_secs(8))
    }

    pub fn new_expire_after(params: P, valid_for: Duration) -> Self {
        Self {
            value: RwLock::new(None),
            params,
            valid_for,
        }
    }

    /// Get the cached value. Value may be re-created if the cache expired.
    /// Warning: Other threads may be blocked trying to call `get`,
    /// until the returned reference is dropped.
    // TODO: Get rid of the `Captures` and use `use<...>` once we have minimum Rust version 1.82.
    pub async fn get(&self) -> impl Deref<Target = T> + Captures<(&(), T)> {
        let cur_time_and_v = self.value.upgradable_read().await;

        match &*cur_time_and_v {
            None => {
                let mut write_time_and_v = RwLockUpgradableReadGuard::upgrade(cur_time_and_v).await;
                let new_value = T::construct(None, &self.params).await;
                *write_time_and_v = Some((Instant::now(), new_value));
                drop(write_time_and_v);

                CacheRefHolder(self.value.read().await)
            }
            Some((cur_time, _v)) => {
                if cur_time.add(self.valid_for) > Instant::now() {
                    let mut write_time_and_v =
                        RwLockUpgradableReadGuard::upgrade(cur_time_and_v).await;
                    let old_value = write_time_and_v.take().unwrap().1;
                    let new_value = T::construct(Some(old_value), &self.params).await;
                    *write_time_and_v = Some((Instant::now(), new_value));
                    drop(write_time_and_v);

                    CacheRefHolder(self.value.read().await)
                } else {
                    CacheRefHolder(RwLockUpgradableReadGuard::downgrade(cur_time_and_v))
                }
            }
        }
    }
}

pub struct CacheRefHolder<'a, T>(RwLockReadGuard<'a, Option<(Instant, T)>>);

#[doc(hidden)]
pub trait Captures<T: ?Sized> {}
impl<T: ?Sized, U: ?Sized> Captures<T> for U {}

impl<T> Deref for CacheRefHolder<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.as_ref().unwrap().1
    }
}

#[allow(async_fn_in_trait)]
pub trait LoadCacheObject {
    type Params;

    async fn construct(previous_value: Option<Self>, params: &Self::Params) -> Self
    where
        Self: Sized;
}
