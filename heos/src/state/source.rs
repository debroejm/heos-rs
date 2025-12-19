//! Stateful source management.

use std::ops::RangeInclusive;
use tokio::sync::{
    Mutex as AsyncMutex,
    RwLockReadGuard as AsyncRwLockReadGuard,
};

use crate::channel::Channel;
use crate::command::browse::*;
use crate::command::{Command, CommandError};
use crate::data::media::{AlbumMetadata, MediaItem, MediaItemsResponse};
use crate::data::option::*;
use crate::data::source::*;
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

    async fn retrieve_all<C>(
        &self,
        cmd_fn: impl Fn(Option<RangeInclusive<usize>>) -> C,
    ) -> Result<WithOptions<Vec<MediaItem>>, CommandError>
    where
        C: Command<Response=WithOptions<MediaItemsResponse>>,
    {
        let response = self.channel.lock().await
            .send_command(cmd_fn(None)).await?;

        let total_count = response.value.count;
        let mut all_items = response.value.items;
        let options = response.options;
        let batch_size = all_items.len();

        while all_items.len() < total_count {
            let current_count = all_items.len();
            let response = self.channel.lock().await
                .send_command(cmd_fn(Some(current_count..=(current_count+batch_size-1)))).await?;
            all_items.extend(response.value.items);
        }

        Ok(WithOptions {
            value: all_items,
            options,
        })
    }

    /// Browse a top-level view of music for this source.
    ///
    /// # Errors
    ///
    /// Errors if sending a [Browse] command errors.
    pub async fn browse(&self) -> Result<WithOptions<Vec<MediaItem>>, CommandError> {
        let source_id = self.data.info.source_id;
        self.retrieve_all(move |range| Browse {
            source_id,
            container_id: None,
            range,
        }).await
    }

    /// Browse a specific container of music for this source.
    ///
    /// This will repeatedly send commands until all music for the specified container is retrieved.
    ///
    /// # Errors
    ///
    /// Errors if sending a [Browse] command errors.
    pub async fn browse_container(
        &self,
        container_id: impl Into<String>,
    ) -> Result<WithOptions<Vec<MediaItem>>, CommandError> {
        let source_id = self.data.info.source_id;
        let container_id = container_id.into();
        self.retrieve_all(move |range| Browse {
            source_id,
            container_id: Some(container_id.clone()),
            range,
        }).await
    }

    /// Browse a specific container of music for this source, limited to the specified range.
    ///
    /// # Errors
    ///
    /// Errors if sending a [Browse] command errors.
    pub async fn browse_container_range(
        &self,
        container_id: impl Into<String>,
        range: RangeInclusive<usize>,
    ) -> Result<WithOptions<MediaItemsResponse>, CommandError> {
        self.channel.lock().await.send_command(Browse {
            source_id: self.data.info.source_id,
            container_id: Some(container_id.into()),
            range: Some(range),
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
    /// This will repeatedly send commands until all music for the specified search is retrieved.
    ///
    /// # Errors
    ///
    /// Errors if sending a [Search] command errors.
    pub async fn search(
        &self,
        search: impl Into<String>,
        criteria: CriteriaId,
    ) -> Result<WithOptions<Vec<MediaItem>>, CommandError> {
        let source_id = self.data.info.source_id;
        let search = search.into();
        self.retrieve_all(move |range| Search {
            source_id,
            search: search.clone(),
            criteria,
            range,
        }).await
    }

    /// Search this source for music, limited to the specified range.
    ///
    /// `criteria` should be a criteria ID yielded by [Self::search_criteria()].
    ///
    /// # Errors
    ///
    /// Errors if sending a [Search] command errors.
    pub async fn search_range(
        &self,
        search: impl Into<String>,
        criteria: CriteriaId,
        range: RangeInclusive<usize>,
    ) -> Result<WithOptions<MediaItemsResponse>, CommandError> {
        self.channel.lock().await.send_command(Search {
            source_id: self.data.info.source_id,
            search: search.into(),
            criteria,
            range: Some(range),
        }).await
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