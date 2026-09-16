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
use diesel::sql_types::Nullable;
use diesel::sql_types::Varchar;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{AddrParseError, Ipv4Addr};
use uuid::Uuid;

use super::constants::UNINIT_POINT_32;

//===================================================================================================

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Position {
    pub fn new() -> Self {
        Position {
            x: UNINIT_POINT_32,
            y: UNINIT_POINT_32,
            z: UNINIT_POINT_32,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.x != UNINIT_POINT_32 && self.y != UNINIT_POINT_32 && self.z != UNINIT_POINT_32
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ {} , {} , {} ]", self.x, self.y, self.z)
    }
}

// Store a String here instead of a Uuid!
#[derive(Debug, Clone, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Varchar)]
pub struct DbUuid(String);

//===================================================================================================

// ToSql now safely borrows the owned String
impl<DB: Backend> ToSql<Varchar, DB> for DbUuid
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        // self.0 is a String. We borrow it, satisfying the 'b lifetime!
        self.0.to_sql(out)
    }
}

// FromSql continues to read a String
impl<DB: Backend> FromSql<Varchar, DB> for DbUuid
where
    String: FromSql<Varchar, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(DbUuid(s))
    }
}

// Convert Uuid -> DbUuid (Happens BEFORE ToSql)
impl From<Uuid> for DbUuid {
    fn from(uuid: Uuid) -> Self {
        // We allocate the String here, so it is owned by DbUuid
        DbUuid(uuid.to_string())
    }
}

// Convert DbUuid -> Uuid (Happens AFTER FromSql)
impl TryFrom<DbUuid> for Uuid {
    type Error = uuid::Error;
    fn try_from(db_uuid: DbUuid) -> Result<Self, Self::Error> {
        Uuid::parse_str(&db_uuid.0)
    }
}

//===================================================================================================

// The transparent bridge struct for DateTime
#[derive(Debug, Clone, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Varchar)]
pub struct DbDateTime(String);

// Tell Diesel how to write to SQLite
impl<DB: Backend> ToSql<Varchar, DB> for DbDateTime
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

// Tell Diesel how to read from SQLite
impl<DB: Backend> FromSql<Varchar, DB> for DbDateTime
where
    String: FromSql<Varchar, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(DbDateTime(s))
    }
}

// Convert DateTime<Utc> -> DbDateTime (Writes RFC3339 string)
impl From<DateTime<Utc>> for DbDateTime {
    fn from(dt: DateTime<Utc>) -> Self {
        DbDateTime(dt.to_rfc3339())
    }
}

// Convert DbDateTime -> DateTime<Utc> (Reads RFC3339 string)
impl TryFrom<DbDateTime> for DateTime<Utc> {
    type Error = chrono::ParseError; // Fulfills Diesel's Error requirement

    fn try_from(db_dt: DbDateTime) -> Result<Self, Self::Error> {
        // Parse from string, then convert from FixedOffset back to Utc
        let fixed_dt = DateTime::parse_from_rfc3339(&db_dt.0)?;
        Ok(fixed_dt.with_timezone(&Utc))
    }
}

//===================================================================================================

// Wrap Option<String> directly
#[derive(Debug, Clone, AsExpression)]
#[diesel(sql_type = Nullable<Varchar>)]
pub struct DbOptDateTime(pub Option<String>);

// Implement Queryable INSTEAD of FromSqlRow to fix the conflict!
impl<DB: Backend> Queryable<Nullable<Varchar>, DB> for DbOptDateTime
where
    Option<String>: Queryable<Nullable<Varchar>, DB>,
{
    type Row = <Option<String> as Queryable<Nullable<Varchar>, DB>>::Row;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        // We let Diesel's built-in Option<String> logic read the row
        let opt_str = Option::<String>::build(row)?;
        Ok(DbOptDateTime(opt_str))
    }
}

// Explicitly tell Diesel how to write this to SQL
impl<DB: Backend> ToSql<Nullable<Varchar>, DB> for DbOptDateTime
where
    Option<String>: ToSql<Nullable<Varchar>, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

// Convert Option<DateTime<Utc>> -> DbOptDateTime (When inserting)
impl From<Option<DateTime<Utc>>> for DbOptDateTime {
    fn from(opt: Option<DateTime<Utc>>) -> Self {
        DbOptDateTime(opt.map(|dt| dt.to_rfc3339()))
    }
}

// Convert DbOptDateTime -> Option<DateTime<Utc>> (When reading via .first() or .load())
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

// Wrap a String that will hold our JSON data
#[derive(Debug, Clone, AsExpression)]
#[diesel(sql_type = Varchar)]
pub struct DbVecString(pub String);

// Tell Diesel how to read this from SQL
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

// Tell Diesel how to write this to SQL
impl<DB: Backend> ToSql<Varchar, DB> for DbVecString
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

// Convert Vec<String> -> DbVecString (When inserting)
impl From<Vec<String>> for DbVecString {
    fn from(vec: Vec<String>) -> Self {
        // Serialize the Vec to a JSON string.
        // We use .expect() here because serializing a simple Vec<String> will never fail.
        let json_string = serde_json::to_string(&vec).expect("Failed to serialize Vec<String>");
        DbVecString(json_string)
    }
}

// Convert DbVecString -> Vec<String> (When reading via .first() or .load())
impl TryFrom<DbVecString> for Vec<String> {
    type Error = serde_json::Error;

    fn try_from(db_vec: DbVecString) -> Result<Self, Self::Error> {
        // Parse the JSON string back into a Vec<String>
        serde_json::from_str(&db_vec.0)
    }
}

//===================================================================================================

// The transparent bridge struct for Ipv4Addr
#[derive(Debug, Clone, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Varchar)]
pub struct DbIpv4Addr(String);

// Tell Diesel how to write to SQLite
impl<DB: Backend> ToSql<Varchar, DB> for DbIpv4Addr
where
    String: ToSql<Varchar, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

// Tell Diesel how to read from SQLite
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
    type Error = AddrParseError; // Fulfills Diesel's Error requirement

    fn try_from(db_ip: DbIpv4Addr) -> Result<Self, Self::Error> {
        db_ip.0.parse()
    }
}
