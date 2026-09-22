// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use chrono::{DateTime, Utc};
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use diesel::sql_types::Nullable;
use diesel::sql_types::Varchar;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{AddrParseError, Ipv4Addr};
use uuid::Uuid;

use super::constants::UNINIT_POINT_32;

//===================================================================================================

/// A coordinate within a 3-dimensional grid.
///
/// Beside the actual coordinates, a position can also express the absence of a coordinate. For
/// this each of the three axes is set to `UNINIT_POINT_32`, which marks the position as invalid.
/// `Default` gives the all-zero position, while `new` gives the invalid one.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Position {
    /// Creates a new position with all axes set to `UNINIT_POINT_32`.
    ///
    /// The resulting position is explicitly invalid, so it can be used as placeholder until the
    /// real coordinates are known.
    ///
    /// # Returns
    ///
    /// A position, for which `is_valid` returns false.
    pub fn new() -> Self {
        Position {
            x: UNINIT_POINT_32,
            y: UNINIT_POINT_32,
            z: UNINIT_POINT_32,
        }
    }

    /// Checks if the position holds a usable coordinate.
    ///
    /// # Returns
    ///
    /// True, if none of the three axes is set to `UNINIT_POINT_32`, else false.
    pub fn is_valid(&self) -> bool {
        self.x != UNINIT_POINT_32 && self.y != UNINIT_POINT_32 && self.z != UNINIT_POINT_32
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ {} , {} , {} ]", self.x, self.y, self.z)
    }
}

/// Bridge-type to store a `Uuid` in a `Varchar`-column.
///
/// The uuid is kept in its already stringified form instead of as `Uuid`, because `ToSql` has to
/// hand out a borrow that lives as long as the surrounding query. Formatting the uuid inside
/// `to_sql` would only produce a temporary, so the string is created once in `From<Uuid>` and
/// owned by this type from then on.
#[derive(Debug, Clone, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Varchar)]
pub struct DbUuid(String);

//===================================================================================================

// writes the owned string, which can be borrowed for the lifetime of the query
impl<DB: Backend> ToSql<Varchar, DB> for DbUuid
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

// reads the column as plain string, without validating it as uuid yet
impl<DB: Backend> FromSql<Varchar, DB> for DbUuid
where
    String: FromSql<Varchar, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(DbUuid(s))
    }
}

// converts on the way into the database, before `to_sql` is called
impl From<Uuid> for DbUuid {
    fn from(uuid: Uuid) -> Self {
        DbUuid(uuid.to_string())
    }
}

// converts on the way out of the database, after `from_sql` has read the column, and so this is
// the place where a malformed value within the database is rejected

impl TryFrom<DbUuid> for Uuid {
    type Error = uuid::Error;
    fn try_from(db_uuid: DbUuid) -> Result<Self, Self::Error> {
        Uuid::parse_str(&db_uuid.0)
    }
}

//===================================================================================================

/// Bridge-type to store a `DateTime<Utc>` in a `Varchar`-column.
///
/// The timestamp is held as RFC-3339 string, for the same lifetime-reason as described for
/// `DbUuid`.
#[derive(Debug, Clone, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Varchar)]
pub struct DbDateTime(String);

impl<DB: Backend> ToSql<Varchar, DB> for DbDateTime
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB: Backend> FromSql<Varchar, DB> for DbDateTime
where
    String: FromSql<Varchar, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(DbDateTime(s))
    }
}

impl From<DateTime<Utc>> for DbDateTime {
    fn from(dt: DateTime<Utc>) -> Self {
        DbDateTime(dt.to_rfc3339())
    }
}

impl TryFrom<DbDateTime> for DateTime<Utc> {
    type Error = chrono::ParseError;

    fn try_from(db_dt: DbDateTime) -> Result<Self, Self::Error> {
        // an RFC-3339 timestamp carries an offset, so the parsed value has to be normalized back
        // to UTC again
        let fixed_dt = DateTime::parse_from_rfc3339(&db_dt.0)?;
        Ok(fixed_dt.with_timezone(&Utc))
    }
}

//===================================================================================================

/// Bridge-type to store an `Option<DateTime<Utc>>` in a nullable `Varchar`-column.
///
/// Works like `DbDateTime`, but keeps the null-case. It implements `Queryable` instead of
/// `FromSqlRow`, because a blanket-impl already provides `FromSqlRow` for `Option<T>` and a
/// second one would collide with it.
#[derive(Debug, Clone, AsExpression)]
#[diesel(sql_type = Nullable<Varchar>)]
pub struct DbOptDateTime(pub Option<String>);

impl<DB: Backend> Queryable<Nullable<Varchar>, DB> for DbOptDateTime
where
    Option<String>: Queryable<Nullable<Varchar>, DB>,
{
    type Row = <Option<String> as Queryable<Nullable<Varchar>, DB>>::Row;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        // the null-handling is left to the already existing impl for Option<String>
        let opt_str = Option::<String>::build(row)?;
        Ok(DbOptDateTime(opt_str))
    }
}

impl<DB: Backend> ToSql<Nullable<Varchar>, DB> for DbOptDateTime
where
    Option<String>: ToSql<Nullable<Varchar>, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

impl From<Option<DateTime<Utc>>> for DbOptDateTime {
    fn from(opt: Option<DateTime<Utc>>) -> Self {
        DbOptDateTime(opt.map(|dt| dt.to_rfc3339()))
    }
}

impl TryFrom<DbOptDateTime> for Option<DateTime<Utc>> {
    type Error = chrono::ParseError;

    fn try_from(db_opt: DbOptDateTime) -> Result<Self, Self::Error> {
        match db_opt.0 {
            Some(s) => {
                let dt = DateTime::parse_from_rfc3339(&s)?;
                Ok(Some(dt.with_timezone(&Utc)))
            }
            None => Ok(None),
        }
    }
}

//===================================================================================================

/// Bridge-type to store a `Vec<String>` in a single `Varchar`-column.
///
/// The list is serialized as JSON-array, so it can be kept in one column instead of requiring an
/// additional table.
#[derive(Debug, Clone, AsExpression)]
#[diesel(sql_type = Varchar)]
pub struct DbVecString(pub String);

impl<DB: Backend> Queryable<Varchar, DB> for DbVecString
where
    String: Queryable<Varchar, DB>,
{
    type Row = <String as Queryable<Varchar, DB>>::Row;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        let s = String::build(row)?;
        Ok(DbVecString(s))
    }
}

impl<DB: Backend> ToSql<Varchar, DB> for DbVecString
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

impl From<Vec<String>> for DbVecString {
    fn from(vec: Vec<String>) -> Self {
        // serializing a Vec<String> has no failure-case, so the error is not propagated here
        let json_string = serde_json::to_string(&vec).expect("Failed to serialize Vec<String>");
        DbVecString(json_string)
    }
}

impl TryFrom<DbVecString> for Vec<String> {
    type Error = serde_json::Error;

    fn try_from(db_vec: DbVecString) -> Result<Self, Self::Error> {
        serde_json::from_str(&db_vec.0)
    }
}

//===================================================================================================

/// Bridge-type to store a VXLAN-VNI in an `Integer`-column.
///
/// A VNI is 24 bit wide, so it fits into the signed 32 bit integer SQLite offers without ever
/// becoming negative. The value is kept as a `u32` everywhere else, because that is what the
/// API and the eBPF-maps use.
#[derive(Debug, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Integer)]
pub struct DbVni(i32);

impl<DB: Backend> ToSql<Integer, DB> for DbVni
where
    i32: ToSql<Integer, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB: Backend> FromSql<Integer, DB> for DbVni
where
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(DbVni(i32::from_sql(bytes)?))
    }
}

impl From<u32> for DbVni {
    fn from(vni: u32) -> Self {
        DbVni(vni as i32)
    }
}

impl From<DbVni> for u32 {
    fn from(db_vni: DbVni) -> Self {
        db_vni.0 as u32
    }
}

/// Bridge-type to store an `Ipv4Addr` in a `Varchar`-column.
///
/// The address is held in its dotted-decimal form, for the same lifetime-reason as described for
/// `DbUuid`.
#[derive(Debug, Clone, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Varchar)]
pub struct DbIpv4Addr(String);

impl<DB: Backend> ToSql<Varchar, DB> for DbIpv4Addr
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB: Backend> FromSql<Varchar, DB> for DbIpv4Addr
where
    String: FromSql<Varchar, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(DbIpv4Addr(s))
    }
}

impl From<Ipv4Addr> for DbIpv4Addr {
    fn from(dt: Ipv4Addr) -> Self {
        DbIpv4Addr(dt.to_string())
    }
}

impl TryFrom<DbIpv4Addr> for Ipv4Addr {
    type Error = AddrParseError;

    fn try_from(db_ip: DbIpv4Addr) -> Result<Self, Self::Error> {
        db_ip.0.parse()
    }
}
