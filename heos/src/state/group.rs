//! Stateful group management.

use tokio::sync::{
    Mutex as AsyncMutex,
    RwLock as AsyncRwLock,
    RwLockReadGuard as AsyncRwLockReadGuard,
};

use crate::command::group::*;
use crate::command::CommandError;
use crate::data::common::*;
use crate::data::group::*;
use crate::data::player::PlayerId;
use crate::channel::Channel;
use crate::state::{locked_data_iter, FromLockedData};

#[derive(Debug)]
pub(super) struct GroupData {
    pub info: GroupInfo,
    pub leader_id: PlayerId,
    pub volume: AsyncRwLock<Volume>,
    pub mute: AsyncRwLock<MuteState>,
}

impl GroupData {
    pub async fn get(channel: &AsyncMutex<Channel>, info: GroupInfo) -> Result<Self, CommandError> {
        let mut leader_id: Option<PlayerId> = None;
        for player in &info.players {
            if player.role == GroupRole::Leader {
                leader_id = Some(player.player_id);
            }
        }
        let leader_id = leader_id
            .ok_or(CommandError::MalformedResponse("Group missing a leader".to_string()))?;

        let volume = channel.lock().await
            .send_command(GetGroupVolume {
                group_id: info.group_id,
            }).await?;
        let mute = channel.lock().await
            .send_command(GetGroupMute {
                group_id: info.group_id,
            }).await?;

        Ok(Self {
            info,
            leader_id,
            volume: AsyncRwLock::new(volume),
            mute: AsyncRwLock::new(mute),
        })
    }
}

/// A snapshot of a group's state.
///
/// This snapshot allows full perusal of a group's state without needing to continually await
/// asynchronous locks.
#[derive(Debug, Clone)]
pub struct GroupSnapshot {
    /// Non-mutable information about the group.
    pub info: GroupInfo,
    /// ID of the group's leader.
    pub leader_id: PlayerId,
    /// The volume level.
    pub volume: Volume,
    /// The mute state.
    pub mute: MuteState,
}

/// Live view into a group's state.
///
/// This provides methods to asynchronously retrieve the latest stateful data, as well as send
/// command requests relevant to this group.
///
/// This view owns a read lock on the list of group states. This means that individual group state
/// (including this group) can be updated when relevant events come in, but
/// [GroupsChanged](crate::data::event::Event::GroupsChanged) events will be delayed until this
/// lock is released.
pub struct Group<'a> {
    channel: &'a AsyncMutex<Channel>,
    data: AsyncRwLockReadGuard<'a, GroupData>,
}

impl<'a> FromLockedData<'a> for Group<'a> {
    type Data = GroupData;

    #[inline]
    fn from_locked_data(
        channel: &'a AsyncMutex<Channel>,
        data: AsyncRwLockReadGuard<'a, Self::Data>,
    ) -> Self {
        Self {
            channel,
            data,
        }
    }
}

impl<'a> Group<'a> {
    /// Get general non-mutable information about this group.
    #[inline]
    pub fn info(&self) -> &GroupInfo {
        &self.data.info
    }

    /// Get the ID of the group's leader.
    #[inline]
    pub fn leader_id(&self) -> PlayerId {
        self.data.leader_id
    }

    /// Retrieve the volume level of this group.
    pub async fn volume(&self) -> Volume {
        *self.data.volume.read().await
    }

    /// Set the volume level of this group.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SetGroupVolume] command errors.
    pub async fn set_volume(&self, level: Volume) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetGroupVolume {
                group_id: self.data.info.group_id,
                level,
            }).await
    }

    /// Increment the volume level of this group.
    ///
    /// # Errors
    ///
    /// Errors if sending a [GroupVolumeUp] command errors.
    pub async fn volume_up(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(GroupVolumeUp {
                group_id: self.data.info.group_id,
                step,
            }).await
    }

    /// Decrement the volume level of this group.
    ///
    /// # Errors
    ///
    /// Errors if sending a [GroupVolumeDown] command errors.
    pub async fn volume_down(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(GroupVolumeDown {
                group_id: self.data.info.group_id,
                step,
            }).await
    }

    /// Retrieve the mute state of this group.
    pub async fn mute(&self) -> MuteState {
        *self.data.mute.read().await
    }

    /// Set the mute state of this group.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SetGroupMute] command errors.
    pub async fn set_mute(&self, state: MuteState) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetGroupMute {
                group_id: self.data.info.group_id,
                state,
            }).await
    }

    /// Toggle the mute state of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [ToggleGroupMute] command errors.
    pub async fn toggle_mute(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(ToggleGroupMute {
                group_id: self.data.info.group_id,
            }).await
    }

    /// Create a snapshot using the latest stateful data of this group.
    ///
    /// This method will retrieve all latest stateful data and put it into a snapshot that can then
    /// be perused without further async awaiting.
    pub async fn snapshot(&self) -> GroupSnapshot {
        GroupSnapshot {
            info: self.data.info.clone(),
            leader_id: self.data.leader_id,
            volume: self.data.volume.read().await.clone(),
            mute: self.data.mute.read().await.clone(),
        }
    }
}

locked_data_iter!(GroupsIter, GroupId, GroupData, Group);