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

#[derive(Debug, Clone)]
pub struct GroupSnapshot {
    pub info: GroupInfo,
    pub leader_id: PlayerId,
    pub volume: Volume,
    pub mute: MuteState,
}

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
    #[inline]
    pub fn info(&self) -> &GroupInfo {
        &self.data.info
    }

    #[inline]
    pub fn leader_id(&self) -> PlayerId {
        self.data.leader_id
    }

    pub async fn volume(&self) -> Volume {
        *self.data.volume.read().await
    }

    pub async fn set_volume(&self, level: Volume) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetGroupVolume {
                group_id: self.data.info.group_id,
                level,
            }).await
    }

    pub async fn volume_up(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(GroupVolumeUp {
                group_id: self.data.info.group_id,
                step,
            }).await
    }

    pub async fn volume_down(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(GroupVolumeDown {
                group_id: self.data.info.group_id,
                step,
            }).await
    }

    pub async fn mute(&self) -> MuteState {
        *self.data.mute.read().await
    }

    pub async fn set_mute(&self, state: MuteState) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetGroupMute {
                group_id: self.data.info.group_id,
                state,
            }).await
    }

    pub async fn toggle_mute(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(ToggleGroupMute {
                group_id: self.data.info.group_id,
            }).await
    }

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