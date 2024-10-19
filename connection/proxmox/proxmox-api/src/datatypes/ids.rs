/* Copyright 2024 Marco Köpcke
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

use std::fmt;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

use log::warn;
use serde::Deserialize;

#[derive(Eq, PartialEq, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct NodeId(String);

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        let mut id_str = value.to_string();
        if id_str.contains('/') {
            warn!("Node ID cannot contain '/'. Trimming it.");
            id_str = id_str.replace('/', "");
        }
        Self(id_str)
    }
}

impl FromStr for NodeId {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('/') {
            Err(crate::Error::InvalidIdValue)
        } else {
            Ok(Self(s.to_string()))
        }
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct VmId(VmIdInner);

impl From<VmId> for u64 {
    fn from(value: VmId) -> Self {
        match value.0 {
            VmIdInner::AsString(v) => v.parse().unwrap(),
            VmIdInner::AsU64(v) => v,
        }
    }
}

impl From<u64> for VmId {
    fn from(value: u64) -> Self {
        Self(VmIdInner::AsU64(value))
    }
}

impl Display for VmId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            VmIdInner::AsString(v) => v.fmt(f),
            VmIdInner::AsU64(v) => v.fmt(f),
        }
    }
}

impl PartialEq for VmId {
    fn eq(&self, other: &Self) -> bool {
        let slf_str = match &self.0 {
            VmIdInner::AsU64(id) => &id.to_string(),
            VmIdInner::AsString(id) => id,
        };
        let oth_str = match &other.0 {
            VmIdInner::AsU64(id) => &id.to_string(),
            VmIdInner::AsString(id) => id,
        };
        slf_str == oth_str
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum VmIdInner {
    AsU64(u64),
    // This is just for deserialization: we trust the Proxmox API to only return strings containing u64s.
    AsString(String),
}
