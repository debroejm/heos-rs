//! Stateful source management.

use std::ops::RangeInclusive;
use tokio::sync::{
    Mutex as AsyncMutex,
    RwLockReadGuard as AsyncRwLockReadGuard,
};

use crate::command::browse::*;
use crate::command::CommandError;
use crate::data::option::*;
use crate::data::source::*;
use crate::channel::Channel;
use crate::state::{locked_data_iter, FromLockedData};

#[derive(Debug)]
pub(super) struct SourceData {
    pub info: SourceInfo,
}

impl SourceData {
    pub async fn get(_channel: &AsyncMutex<Channel>, info: SourceInfo) -> Result<Self, CommandError> {
        // TODO: does this need 'channel'?
        Ok(Self {
            info,
        })
    }
}

/// Live view into a source's state.
///
/// This provides methods to asynchronously retrieve the latest stateful data, as well as send
/// command requests relevant to this source.
///
/// This view owns a read lock on the list of source states. This means that individual source state
/// (including this source) can be updated when relevant events come in, but
/// [SourcesChanged](crate::data::event::Event::SourcesChanged) events will be delayed until this
/// lock is released.
pub struct Source<'a> {
    channel: &'a AsyncMutex<Channel>,
    data: AsyncRwLockReadGuard<'a, SourceData>,
}

impl<'a> FromLockedData<'a> for Source<'a> {
    type Data = SourceData;

    #[inline]
    fn from_locked_data(
        channel: &'a AsyncMutex<Channel>,
        data: AsyncRwLockReadGuard<'a, Self::Data>
    ) -> Self
    where
        Self: 'a,
    {
        Self {
            channel,
            data,
        }
    }
}

impl<'a> Source<'a> {
    /// Get general non-mutable information about this source.
    #[inline]
    pub fn info(&self) -> &SourceInfo {
        &self.data.info
    }

    /// Browse a top-level view of music for this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [Browse] command errors.
    pub async fn browse(&self) -> Result<WithOptions<Vec<SourceItem>>, CommandError> {
        self.channel.lock().await
            .send_command(Browse {
                source_id: self.data.info.source_id,
                container_id: None,
                range: None,
            }).await
    }

    /// Browse a specific container of music for this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [Browse] command errors.
    pub async fn browse_container(
        &self,
        container_id: impl Into<String>,
        range: Option<RangeInclusive<usize>>
    ) -> Result<WithOptions<Vec<SourceItem>>, CommandError> {
        self.channel.lock().await
            .send_command(Browse {
                source_id: self.data.info.source_id,
                container_id: Some(container_id.into()),
                range,
            }).await
    }

    /// Retrieve valid search criteria for this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [GetSearchCriteria] command errors.
    pub async fn search_criteria(&self) -> Result<Vec<SearchCriteria>, CommandError> {
        self.channel.lock().await
            .send_command(GetSearchCriteria {
                source_id: self.data.info.source_id,
            }).await
    }

    /// Search this source for music.
    ///
    /// `criteria` should be a criteria ID yielded by [Self::search_criteria()].
    ///
    /// # Errors
    ///
    /// Errors if sending a [Search] command errors.
    pub async fn search(
        &self,
        search: impl Into<String>,
        criteria: impl Into<String>,
    ) -> Result<WithOptions<Vec<SourceItem>>, CommandError> {
        self.search_impl(search.into(), criteria.into()).await
    }

    async fn search_impl(
        &self,
        search: String,
        criteria: String,
    ) -> Result<WithOptions<Vec<SourceItem>>, CommandError> {
        let mut all_items = vec![];
        let mut options = vec![];
        loop {
            let response = self.channel.lock().await
                .send_command(Search {
                    source_id: self.data.info.source_id,
                    search: search.clone(),
                    criteria: criteria.clone(),
                    range: None,
                }).await?;

            if options.is_empty() {
                options = response.options;
            }
            let results = response.value;

            if results.source_items.is_empty() {
                break
            }

            all_items.extend(results.source_items);
            if results.count != 0 && all_items.len() >= results.count {
                break
            }
        }
        Ok(WithOptions {
            value: all_items,
            options,
        })
    }

    /// Rename a playlist belonging to this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [RenamePlaylist] command errors.
    pub async fn rename_playlist(
        &self,
        container_id: impl Into<String>,
        new_name: impl Into<String>,
    ) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(RenamePlaylist {
                source_id: self.data.info.source_id,
                container_id: container_id.into(),
                name: new_name.into(),
            }).await
    }

    /// Delete a playlist belonging to this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [DeletePlaylist] command errors.
    pub async fn delete_playlist(
        &self,
        container_id: impl Into<String>,
    ) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(DeletePlaylist {
                source_id: self.data.info.source_id,
                container_id: container_id.into(),
            }).await
    }

    /// Retrieve album metadata for an album that comes from this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [GetAlbumMetadata] command errors.
    pub async fn album_metadata(
        &self,
        container_id: impl Into<String>,
    ) -> Result<Vec<AlbumMetadata>, CommandError> {
        self.channel.lock().await
            .send_command(GetAlbumMetadata {
                source_id: self.data.info.source_id,
                container_id: container_id.into(),
            }).await
    }

    /// Set a [ServiceOption] associated with this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SetServiceOption] command errors.
    pub async fn set_service_option(
        &self,
        option: ServiceOption,
    ) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetServiceOption {
                source_id: self.data.info.source_id,
                option,
            }).await
    }
}

locked_data_iter!(SourcesIter, SourceId, SourceData, Source);