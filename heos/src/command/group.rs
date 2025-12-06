//! Commands to query/control groups of players.
//!
//! See [crate::data::group] for more information on groups.

use serde::Serialize;

use crate::command::impl_command;
use crate::data::common::*;
use crate::data::group::*;
use crate::data::player::*;

/// Retrieve all known groups.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::GetGroups;
/// use heos::data::group::GroupInfo;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let groups: Vec<GroupInfo> = heos.command(GetGroups::default()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Default, Clone, Copy)]
pub struct GetGroups {}
impl_command!(GetGroups, "group", "get_groups", Vec<GroupInfo>);

/// Retrieve a specific group by [ID](GroupId).
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::GetGroupInfo;
/// use heos::data::group::{GroupId, GroupInfo};
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let group: GroupInfo = heos.command(GetGroupInfo {
///     group_id: GroupId::from(1),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetGroupInfo {
    /// ID of the group to retrieve.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(GetGroupInfo, "group", "get_group_info", GroupInfo);

/// Set members of a group.
///
/// This command can be used in a couple of different ways. The first ID in `player_ids` should
/// always be the group leader's ID.
///
/// If the group leader's ID is not currently part of a group, a new group will be created and the
/// remaining specified IDs will be added to that group.
///
/// If the group leader's ID refers to an existing group, the remaining IDs will be used to set the
/// remaining members of that group. For example, if the group \[A,B,C\] exists, and SetGroup is
/// executed with the parameter \[A,B,D\], then the existing group will have C removed and D added.
///
/// If a single group leader ID is specified with no following IDs, all players in the group will be
/// ungrouped and the group will be deleted.
///
/// When adding members to a group, it is recommended to first ensure any new members are not
/// currently in a prior group, as that can cause the command to fail under certain circumstances.
///
/// # Examples
///
/// Create a new group
/// ```
/// use assert_matches::assert_matches;
/// use assert_unordered::assert_eq_unordered;
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::{GetGroupInfo, SetGroup};
/// use heos::data::group::*;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let result = heos.command(SetGroup {
///     player_ids: vec![
///         // Neither of these are in an existing group. PlayerId '42' will be the new group leader.
///         PlayerId::from(42),
///         PlayerId::from(43),
///     ],
/// }).await?;
/// let group_id = assert_matches!(result, SetGroupResult::CreatedOrModified { group_id, .. } => group_id);
/// let group = heos.command(GetGroupInfo { group_id }).await?;
/// assert_eq_unordered!(group.players, vec![
///     GroupPlayer {
///         name: "Player42".to_string(),
///         player_id: PlayerId::from(42),
///         role: GroupRole::Leader,
///     },
///     GroupPlayer {
///         name: "Player43".to_string(),
///         player_id: PlayerId::from(43),
///         role: GroupRole::Member,
///     },
/// ]);
/// # Ok(())
/// # }
/// ```
///
/// Modify members of an existing group
/// ```
/// use assert_matches::assert_matches;
/// use assert_unordered::assert_eq_unordered;
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::{GetGroupInfo, SetGroup};
/// use heos::data::group::*;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let result = heos.command(SetGroup {
///     // Given the existing group [1,2,3]:
///     player_ids: vec![
///         // '1' is the existing group leader
///         PlayerId::from(1),
///         // Keep '3', but remove '2' and add '43'
///         PlayerId::from(3),
///         PlayerId::from(43),
///     ],
/// }).await?;
/// let group_id = assert_matches!(result, SetGroupResult::CreatedOrModified { group_id, .. } => group_id);
/// let group = heos.command(GetGroupInfo { group_id }).await?;
/// assert_eq_unordered!(group.players, vec![
///     GroupPlayer {
///         name: "Player1".to_string(),
///         player_id: PlayerId::from(1),
///         role: GroupRole::Leader,
///     },
///     GroupPlayer {
///         name: "Player3".to_string(),
///         player_id: PlayerId::from(3),
///         role: GroupRole::Member,
///     },
///     GroupPlayer {
///         name: "Player43".to_string(),
///         player_id: PlayerId::from(43),
///         role: GroupRole::Member,
///     },
/// ]);
/// # Ok(())
/// # }
/// ```
///
/// Delete a group
/// ```
/// use assert_matches::assert_matches;
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::{GetGroupInfo, SetGroup};
/// use heos::data::group::*;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let result = heos.command(SetGroup {
///     player_ids: vec![
///         // Delete the group that '1' leads
///         PlayerId::from(1),
///     ],
/// }).await?;
/// assert_matches!(result, SetGroupResult::Deleted);
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone)]
pub struct SetGroup {
    #[serde(rename = "pid")]
    pub player_ids: Vec<PlayerId>,
}
impl_command!(SetGroup, "group", "set_group", SetGroupResult);

/// Retrieve a group's current volume level.
///
/// Note that this is different from an individual player's volume level.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::GetGroupVolume;
/// use heos::data::common::Volume;
/// use heos::data::group::GroupId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let volume: Volume = heos.command(GetGroupVolume {
///     group_id: GroupId::from(1),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetGroupVolume {
    /// ID of the group to retrieve.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(GetGroupVolume, "group", "get_volume", Volume);

/// Set a group's volume level.
///
/// Note that this is different from an individual player's volume level.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::SetGroupVolume;
/// use heos::data::common::Volume;
/// use heos::data::group::GroupId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SetGroupVolume {
///     group_id: GroupId::from(1),
///     level: Volume::try_from(33).unwrap(),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetGroupVolume {
    /// ID of the group to set.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    /// New volume level to set.
    pub level: Volume,
}
impl_command!(SetGroupVolume, "group", "set_volume", ());

/// Increment a group's volume level.
///
/// Note that this is different from an individual player's volume level.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::GroupVolumeUp;
/// use heos::data::group::GroupId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(GroupVolumeUp {
///     group_id: GroupId::from(1),
///     step: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GroupVolumeUp {
    /// ID of the group to increment.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    /// How much to increment by.
    ///
    /// This parameter may be omitted (`None`), and defaults to an increment step of `5`.
    pub step: Option<VolumeStep>,
}
impl_command!(GroupVolumeUp, "group", "volume_up", ());

/// Decrement a group's volume level.
///
/// Note that this is different from an individual player's volume level.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::GroupVolumeDown;
/// use heos::data::common::VolumeStep;
/// use heos::data::group::GroupId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(GroupVolumeDown {
///     group_id: GroupId::from(1),
///     step: Some(VolumeStep::try_from(7).unwrap()),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GroupVolumeDown {
    /// ID of the group to decrement.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    /// How much to decrement by.
    ///
    /// This parameter may be omitted (`None`), and defaults to a decrement step of `5`.
    pub step: Option<VolumeStep>,
}
impl_command!(GroupVolumeDown, "group", "volume_down", ());

/// Retrieve a group's [MuteState].
///
/// Note that this is different from an individual player's [MuteState].
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::GetGroupMute;
/// use heos::data::common::MuteState;
/// use heos::data::group::GroupId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let mute_state: MuteState = heos.command(GetGroupMute {
///     group_id: GroupId::from(1),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetGroupMute {
    /// ID of the group to retrieve.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(GetGroupMute, "group", "get_mute", MuteState);

/// Set a group's [MuteState].
///
/// Note that this is different from an individual player's [MuteState].
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::SetGroupMute;
/// use heos::data::common::MuteState;
/// use heos::data::group::GroupId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SetGroupMute {
///     group_id: GroupId::from(1),
///     state: MuteState::On,
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetGroupMute {
    /// ID of the group to set.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    /// New MuteState to set.
    pub state: MuteState,
}
impl_command!(SetGroupMute, "group", "set_mute", ());

/// Toggle a group's [MuteState].
///
/// If it was previously off, it will be toggled to be on, and vice versa.
///
/// Note that this is different from an individual player's [MuteState].
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::group::ToggleGroupMute;
/// use heos::data::group::GroupId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(ToggleGroupMute {
///     group_id: GroupId::from(1),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct ToggleGroupMute {
    /// ID of the group to toggle.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(ToggleGroupMute, "group", "toggle_mute", ());