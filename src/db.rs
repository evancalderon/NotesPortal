use aws_config::meta::region::RegionProviderChain;
use std::collections::HashMap;

use crate::counter::Counter;
use aws_sdk_dynamodb::{
    operation::scan::ScanOutput,
    types::{AttributeDefinition, AttributeValue, KeySchemaElement, ProvisionedThroughput},
};
use chrono::{DateTime, Duration, Local};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_diff::{Diff, SerdeDiff};

#[derive(SerdeDiff, Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub enum UserRole {
    Standard,
    Admin,
}

#[derive(SerdeDiff, Deserialize, Serialize, Clone, Debug)]
pub struct User {
    pub name: String,
    pub primary_key: String,
    pub hash: u64,
    pub role: UserRole,
}

#[derive(SerdeDiff, Deserialize, Serialize, Clone, Debug)]
pub struct Session {
    pub token: String,
    pub expires: i64,
    pub user: User,
}

#[derive(SerdeDiff, Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct Note {
    pub id: u32,
    pub date: String,
    pub user: String,
    pub content: String,
}

#[derive(SerdeDiff, Deserialize, Serialize, Clone, Debug)]
pub struct Student {
    pub first_name: String,
    pub last_name: String,
    pub id: String,
    pub name: String,
    #[serde_diff(opaque)]
    pub date: Option<DateTime<Local>>,
    pub time: Option<String>,
    pub belt: String,
    pub logins: Vec<Note>,
    pub notes: Vec<Note>,
    pub behaviours: Vec<Note>,
    pub assigned: Option<String>,
    pub note_counter: Counter<u32>,
}

#[derive(SerdeDiff, Deserialize, Serialize, Clone, Debug)]
pub struct StudentImportedInfo {
    pub name: String,
    pub belt: String,
    pub logins: Vec<String>,
    pub notes: Vec<String>,
    pub behaviours: Vec<String>,
}

pub trait Database<'a, ColumnType> {
    fn save(&self);
    #[deprecated]
    fn column<'b: 'a>(&'b self, name: &str) -> ColumnType;
}

pub trait Column<DatabaseType, ColumnRefType> {
    async fn get_values<S: DeserializeOwned + PrimaryKeyName>(
        &self,
        db: &DatabaseType,
    ) -> Result<Vec<S>, String>;
    async fn get<S: DeserializeOwned + PrimaryKeyName>(
        &self,
        db: &DatabaseType,
        k: &str,
    ) -> Result<Option<S>, String>;
    async fn put<S: Serialize + PrimaryKeyName>(
        &self,
        db: &DatabaseType,
        k: &str,
        v: S,
    ) -> Result<(), String>;
    #[deprecated]
    async fn get_update<S: SerdeDiff + Serialize + DeserializeOwned + Clone + PrimaryKeyName>(
        &self,
        db: &DatabaseType,
        k: &str,
        f: impl FnOnce(&mut S) -> (),
    ) -> Result<(), String>;
    async fn diff_update<S: SerdeDiff + Serialize + DeserializeOwned + Clone + PrimaryKeyName>(
        &self,
        db: &DatabaseType,
        k: &str,
        v: &S,
        f: impl FnOnce(&mut S) -> (),
    ) -> Result<(), String>;
    async fn delete<S: PrimaryKeyName>(&self, db: &DatabaseType, k: &str) -> Result<(), String>;
}

pub struct DynamoDB {
    internal: aws_sdk_dynamodb::Client,
}

pub struct DynamoDBColumn {
    table_name: String,
}

pub trait PrimaryKeyName {
    fn get_primary_key_name() -> &'static str;
}

pub trait PrimaryKeyValue<DT> {
    fn get_primary_key_value(&self) -> DT;
}

impl PrimaryKeyName for User {
    fn get_primary_key_name() -> &'static str {
        return "primary_key";
    }
}

impl PrimaryKeyValue<String> for User {
    fn get_primary_key_value(&self) -> String {
        self.name.clone()
    }
}

impl PrimaryKeyName for Session {
    fn get_primary_key_name() -> &'static str {
        return "token";
    }
}

impl PrimaryKeyValue<String> for Session {
    fn get_primary_key_value(&self) -> String {
        self.token.clone()
    }
}

impl PrimaryKeyName for Student {
    fn get_primary_key_name() -> &'static str {
        return "id";
    }
}

impl PrimaryKeyValue<String> for Student {
    fn get_primary_key_value(&self) -> String {
        self.id.clone()
    }
}

impl PrimaryKeyName for StudentImportedInfo {
    fn get_primary_key_name() -> &'static str {
        return "name";
    }
}

impl PrimaryKeyValue<String> for StudentImportedInfo {
    fn get_primary_key_value(&self) -> String {
        self.name.clone()
    }
}

impl<S: PrimaryKeyName> PrimaryKeyName for Diff<'_, '_, S> {
    fn get_primary_key_name() -> &'static str {
        S::get_primary_key_name()
    }
}

impl<S: PrimaryKeyName> PrimaryKeyName for &S {
    fn get_primary_key_name() -> &'static str {
        S::get_primary_key_name()
    }
}

impl DynamoDB {
    pub async fn new(test_db: bool) -> Self {
        let region_provider = RegionProviderChain::default_provider().or_else("us-west-1");
        let mut env = aws_config::from_env().region(region_provider);
        if test_db {
            env = env.endpoint_url("http://127.0.0.1:8000");
        }
        Self {
            internal: aws_sdk_dynamodb::Client::new(&env.load().await),
        }
    }

    pub async fn create_table<S: PrimaryKeyName>(&self, table: &str) {
        let result = self
            .internal
            .describe_table()
            .table_name(table)
            .send()
            .await;

        if result.is_err() {
            let ad = AttributeDefinition::builder()
                .attribute_name(S::get_primary_key_name())
                .attribute_type(aws_sdk_dynamodb::types::ScalarAttributeType::S)
                .build();
            let ks = KeySchemaElement::builder()
                .attribute_name(S::get_primary_key_name())
                .key_type(aws_sdk_dynamodb::types::KeyType::Hash)
                .build();
            let pt = ProvisionedThroughput::builder()
                .read_capacity_units(10)
                .write_capacity_units(5)
                .build();
            let res = self
                .internal
                .create_table()
                .table_name(table)
                .attribute_definitions(ad)
                .key_schema(ks)
                .provisioned_throughput(pt)
                .send()
                .await;
            if res.is_err() {
                println!("{:?}", res)
            }
        }
    }
}

impl<'a> Database<'a, DynamoDBColumn> for DynamoDB {
    fn save(&self) {}

    fn column<'b: 'a>(&'b self, name: &str) -> DynamoDBColumn {
        DynamoDBColumn {
            table_name: name.to_string(),
        }
    }
}

impl Column<DynamoDB, String> for DynamoDBColumn {
    async fn get_values<S: DeserializeOwned + PrimaryKeyName>(
        &self,
        db: &DynamoDB,
    ) -> Result<Vec<S>, String> {
        let mut result: ScanOutput;
        let mut next_handle: Option<(&String, &AttributeValue)> = None;
        let mut results: Vec<S> = vec![];
        loop {
            let mut scan = db.internal.scan().table_name(self.table_name.to_string());
            if let Some(next_handle) = next_handle {
                scan = scan.exclusive_start_key(next_handle.0, next_handle.1.clone())
            }

            result = scan.send().await.map_err(|e| e.to_string())?;
            next_handle = result.last_evaluated_key().and_then(|v| v.iter().next());

            let mut items: Vec<S> =
                serde_dynamo::from_items(result.items().map(|s| s.to_vec()).unwrap())
                    .map_err(|e| e.to_string())?;
            results.append(&mut items);
            if next_handle.is_none() {
                break;
            }
        }

        Ok(results)
    }

    async fn get<S: DeserializeOwned + PrimaryKeyName>(
        &self,
        db: &DynamoDB,
        k: &str,
    ) -> Result<Option<S>, String> {
        let get_item = db
            .internal
            .get_item()
            .table_name(self.table_name.clone())
            .set_key(Some(HashMap::from([(
                String::from(S::get_primary_key_name()),
                serde_dynamo::to_attribute_value(String::from(k)).map_err(|e| e.to_string())?,
            )])));

        let result = get_item.send().await.map_err(|e| e.to_string())?;

        if let Some(item) = result.item() {
            let item: S = serde_dynamo::from_item(item.clone()).map_err(|e| e.to_string())?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn put<S: Serialize + PrimaryKeyName>(
        &self,
        db: &DynamoDB,
        _k: &str,
        v: S,
    ) -> Result<(), String> {
        let item = serde_dynamo::to_item(v);
        let put_item = db
            .internal
            .put_item()
            .table_name(self.table_name.clone())
            .set_item(Some(item.map_err(|e| e.to_string())?));
        let res = put_item.send().await;
        if let Err(e) = res {
            println!("{:?}", e);
        }
        Ok(())
    }

    async fn get_update<S: Serialize + DeserializeOwned + Clone + PrimaryKeyName>(
        &self,
        db: &DynamoDB,
        k: &str,
        f: impl FnOnce(&mut S) -> (),
    ) -> Result<(), String> {
        let v: Option<S> = self.get::<S>(db, k).await?;
        if let Some(mut v) = v {
            f(&mut v);
            self.put(db, k, v).await?;
        }

        Ok(())
    }

    async fn diff_update<S: SerdeDiff + Serialize + DeserializeOwned + Clone + PrimaryKeyName>(
        &self,
        db: &DynamoDB,
        k: &str,
        v: &S,
        f: impl FnOnce(&mut S) -> (),
    ) -> Result<(), String>
    where
        S: PrimaryKeyName,
    {
        let mut val = v.clone();
        f(&mut val);
        self.put(&db, k, &val).await?;

        Ok(())
    }

    async fn delete<S: PrimaryKeyName>(&self, db: &DynamoDB, k: &str) -> Result<(), String> {
        db.internal
            .delete_item()
            .table_name(self.table_name.clone())
            .key(
                S::get_primary_key_name(),
                serde_dynamo::to_attribute_value(k).map_err(|e| e.to_string())?,
            )
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

struct CachedValue<DT: SerdeDiff + Serialize + DeserializeOwned + Clone + PrimaryKeyName> {
    value: DT,
    cached_at: DateTime<Local>,
}

impl<DT: SerdeDiff + Serialize + DeserializeOwned + Clone + PrimaryKeyName> From<DT>
    for CachedValue<DT>
{
    fn from(value: DT) -> Self {
        Self {
            value,
            cached_at: Local::now(),
        }
    }
}

pub struct CachingDynamoDBColumn<
    DT: SerdeDiff + Serialize + DeserializeOwned + Clone + PrimaryKeyName,
> {
    internal: DynamoDBColumn,
    cached: HashMap<String, CachedValue<DT>>,
    got_values: Option<DateTime<Local>>,
    expiration_time: Duration,
}

#[allow(dead_code)]
impl<DT> CachingDynamoDBColumn<DT>
where
    DT: SerdeDiff + Serialize + DeserializeOwned + Clone + PrimaryKeyName + PrimaryKeyValue<String>,
{
    pub fn from(col: DynamoDBColumn) -> Self {
        Self {
            internal: col,
            cached: HashMap::new(),
            got_values: None,
            expiration_time: Duration::minutes(30),
        }
    }

    pub async fn get_values(&mut self, db: &DynamoDB) -> Result<Vec<DT>, String> {
        let result = self.internal.get_values::<DT>(&db).await;
        if self
            .got_values
            .is_some_and(|v| Local::now() - v < self.expiration_time)
        {
            return result;
        } else {
            if let Ok(v) = result.clone() {
                self.got_values = Some(Local::now());
                self.cached.extend(
                    v.iter()
                        .map(|v| (v.get_primary_key_value(), v.clone().into())),
                );
            }
            result
        }
    }

    pub async fn get(&mut self, db: &DynamoDB, k: &str) -> Result<Option<DT>, String> {
        let v = self.cached.get(k);
        if v.is_some_and(|v| Local::now() - v.cached_at < self.expiration_time) {
            return Ok(Some(v.unwrap().value.clone()));
        } else {
            let result = self.internal.get::<DT>(&db, k.clone()).await;
            if let Ok(Some(v)) = result.clone() {
                self.cached.insert(k.to_string(), v.into());
            }
            result
        }
    }

    pub async fn put(&mut self, db: &DynamoDB, _k: &str, v: DT) -> Result<(), String> {
        let result = self.internal.put::<DT>(&db, _k, v.clone()).await;
        if result.is_ok() {
            self.cached.insert(_k.to_string(), v.clone().into());
        }
        result
    }

    pub async fn delete(&mut self, db: &DynamoDB, k: &str) -> Result<(), String> {
        let result = self.internal.delete::<DT>(&db, k).await;
        if result.is_ok() {
            self.cached.remove(k);
        }
        result
    }

    pub async fn get_update(
        &mut self,
        db: &DynamoDB,
        k: &str,
        f: impl FnOnce(&mut DT) -> (),
    ) -> Result<(), String> {
        let v = self.get(&db, k).await?;
        if let Some(mut v) = v {
            f(&mut v);
            self.put(&db, k, v).await?;
        }
        Ok(())
    }

    pub async fn diff_update(
        &mut self,
        db: &DynamoDB,
        k: &str,
        v: &DT,
        f: impl FnOnce(&mut DT) -> (),
    ) -> Result<(), String> {
        let mut mv = v.clone();
        f(&mut mv);
        self.put(&db, k, mv.clone()).await?;
        
        Ok(())
    }
}
