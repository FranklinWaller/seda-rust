use std::sync::Arc;

use tokio::sync::Mutex as AsyncMutex;
use tokio_rusqlite::Connection;

use super::RuntimeError;

#[async_trait::async_trait]
pub trait DatabaseAdapter: Send {
    async fn connect(&mut self) -> Result<Connection, RuntimeError>;
    async fn set(&mut self, conn: Connection, key: &str, value: &str) -> Result<(), RuntimeError>;
    async fn get(&self, conn: Connection, key: &str) -> Result<Option<String>, RuntimeError>;
}

#[async_trait::async_trait]
pub trait HttpAdapter: Send {
    // TODO: add headers + methods
    async fn fetch(&mut self, url: &str) -> Result<reqwest::Response, reqwest::Error>;
}

pub trait DummyAdapter: Clone + Default + 'static + Send + Sync {
    fn get(&self, key: &str);
}

pub trait VmAdapterTypes: Clone + Default + 'static + Send + Sync {
    // Put memory etc here, only sync traits
    type Dummy: DummyAdapter;
}

#[derive(Default)]
pub struct VmAdapters<VmTypes>
where
    VmTypes: VmAdapterTypes,
{
    pub memory: VmTypes::Dummy,
}
pub trait HostAdapterTypes: Default + Clone {
    type Database: DatabaseAdapter + Default + Clone;
    type Http: HttpAdapter + Default + Clone;
}

#[derive(Default, Clone)]
pub struct HostAdaptersInner<T>
where
    T: HostAdapterTypes,
{
    pub database: Arc<AsyncMutex<T::Database>>,
    pub http:     Arc<AsyncMutex<T::Http>>,
}

#[derive(Clone)]
pub struct HostAdapters<T>
where
    T: HostAdapterTypes,
{
    inner: HostAdaptersInner<T>,
}

impl<T> HostAdapters<T>
where
    T: HostAdapterTypes,
{
    pub fn new() -> Self {
        Self {
            inner: HostAdaptersInner::<T>::default(),
        }
    }

    pub fn db_get(&self, conn: Connection, key: &str) -> Result<Option<String>, RuntimeError> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current()
                .block_on(async move { self.inner.database.lock().await.get(conn, key).await })
        })
    }

    pub fn db_set(&self, conn: Connection, key: &str, value: &str) -> Result<(), RuntimeError> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current()
                .block_on(async move { self.inner.database.lock().await.set(conn, key, value).await })
        })
    }

    pub fn db_connect(&self) -> Result<Connection, RuntimeError> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async move { self.inner.database.lock().await.connect().await })
        })
    }

    pub fn http_fetch(&self, url: &str) -> Result<String, reqwest::Error> {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current()
                .block_on(async move { self.inner.http.lock().await.fetch(url).await?.text().await })
        })
    }
}

impl<T> Default for HostAdapters<T>
where
    T: HostAdapterTypes,
{
    fn default() -> Self {
        Self::new()
    }
}
