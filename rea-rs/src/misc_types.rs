use reaper_medium::PositionInSeconds;
use serde_derive::{Deserialize, Serialize};

use crate::{Direction, Project, Reaper};
use std::{mem::MaybeUninit, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}
impl Color {
    /// New color from r, g, b (0..255).
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Get as tuple.
    pub fn get(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    /// Make from the OS-dependent color.
    pub fn from_native(native: i32) -> Self {
        unsafe {
            let low = Reaper::get().low();
            let (mut r, mut g, mut b) = (
                MaybeUninit::new(0),
                MaybeUninit::new(0),
                MaybeUninit::new(0),
            );
            low.ColorFromNative(
                native,
                r.as_mut_ptr(),
                g.as_mut_ptr(),
                b.as_mut_ptr(),
            );
            Self {
                r: r.assume_init_read() as u8,
                g: g.assume_init_read() as u8,
                b: b.assume_init_read() as u8,
            }
        }
    }

    /// Convert to OS-dependent color.
    pub fn to_native(&self) -> i32 {
        let low = Reaper::get().low();
        low.ColorToNative(self.r as i32, self.g as i32, self.b as i32)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize,
)]
pub struct SampleAmount {
    amount: u32,
}
impl SampleAmount {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
    pub fn get(&self) -> u32 {
        self.amount
    }
    pub fn from_time(time: Duration, samplerate: u32) -> Self {
        let amount = time.as_micros() * samplerate as u128 / 1_000_000;
        Self {
            amount: amount as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct HardwareSocket {
    index: u32,
    name: String,
}
impl HardwareSocket {
    pub(crate) fn new(index: u32, name: String) -> Self {
        Self { index, name }
    }
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::SampleAmount;

    #[test]
    fn test_sample_amount() {
        assert_eq!(
            SampleAmount::from_time(Duration::from_secs(1), 44100).get(),
            44100
        );
    }
}

/// Position in project.
///
/// Internally holds only [Duration] from project start.
/// Keeps interfaces to all time transformations (e.g. between secs, quarters
/// and ppq)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    as_duration: Duration,
}
impl From<f64> for Position {
    fn from(value: f64) -> Self {
        let duration = Duration::from_secs_f64(value);
        Self {
            as_duration: duration,
        }
    }
}
impl From<PositionInSeconds> for Position {
    fn from(value: PositionInSeconds) -> Self {
        Self::from(value.get())
    }
}
impl Into<f64> for Position {
    fn into(self) -> f64 {
        self.as_duration.as_secs_f64()
    }
}
impl From<Duration> for Position {
    fn from(value: Duration) -> Self {
        Self { as_duration: value }
    }
}
impl Into<Duration> for Position {
    fn into(self) -> Duration {
        self.as_duration()
    }
}
impl Position {
    pub fn new(duration_from_project_start: Duration) -> Self {
        Self {
            as_duration: duration_from_project_start,
        }
    }
    pub fn as_duration(&self) -> Duration {
        self.as_duration
    }
    pub fn as_quarters(&self, project: &Project) -> f64 {
        unsafe {
            Reaper::get().low().TimeMap2_timeToQN(
                project.context().to_raw(),
                self.as_duration().as_secs_f64(),
            )
        }
    }
    pub fn from_quarters(quarters: impl Into<f64>, project: &Project) -> Self {
        unsafe {
            Self::from(Reaper::get().low().TimeMap2_QNToTime(
                project.context().to_raw(),
                quarters.into(),
            ))
        }
    }
    // pub fn as_ppq(&self, take: Take) -> u32 {
    //     unsafe {
    //         Reaper::get().low().MIDI_GetPPQPosFromProjTime(
    //             take.get().as_mut(),
    //             self.as_duration().as_secs_f64(),
    //         ) as u32
    //     }
    // }

    // pub fn from_ppq(ppq: impl Into<u32>, take: Take) -> Self {
    //     unsafe {
    //         Self::from(Reaper::get().low().MIDI_GetProjTimeFromPPQPos(
    //             take.get().as_mut(),
    //             ppq.into() as f64,
    //         ))
    //     }
    // }
}

/// Project playback rate.
///
/// Normally, represents multiplication factor to project tempo.
///
/// Can be normalized into slider range values (0.0 .. 1.0)
///
/// # Example
/// ```no_run
/// use rea_rs::PlayRate;
/// let plrt = PlayRate::from(0.25);
/// assert_eq!(plrt.normalized(), 0.0);
///
/// let plrt = PlayRate::from(4.0);
/// assert_eq!(plrt.normalized(), 1.0);
///
/// let plrt = PlayRate::from(1.0);
/// assert_eq!(plrt.normalized(), 0.2);
///
/// let plrt = PlayRate::from(2.5);
/// assert_eq!(plrt.normalized(), 0.6);
/// ```
///
/// [Project::get_play_rate]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PlayRate {
    raw: f64,
}
impl PlayRate {
    /// Convert to slider value.
    pub fn normalized(&self) -> f64 {
        Reaper::get()
            .low()
            .Master_NormalizePlayRate(self.raw, false)
    }

    /// Create from slider value.
    pub fn from_normalized(value: f64) -> Self {
        let raw = Reaper::get().low().Master_NormalizePlayRate(value, true);
        PlayRate::from(raw)
    }
}
impl From<f64> for PlayRate {
    fn from(value: f64) -> Self {
        PlayRate { raw: value }
    }
}
impl Into<f64> for PlayRate {
    fn into(self) -> f64 {
        self.raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeRangeKind {
    TimeSelection,
    LoopSelection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimeRange<'a> {
    project: &'a Project,
    kind: TimeRangeKind,
}
impl<'a> TimeRange<'a> {
    pub fn new(project: &'a Project, kind: TimeRangeKind) -> Self {
        Self { project, kind }
    }

    pub fn get_kind(&self) -> TimeRangeKind {
        return self.kind;
    }

    pub fn get(&self) -> (Position, Position) {
        unsafe {
            let is_loop = match self.kind {
                TimeRangeKind::LoopSelection => true,
                TimeRangeKind::TimeSelection => false,
            };
            let (mut start, mut end) =
                (MaybeUninit::zeroed(), MaybeUninit::zeroed());
            Reaper::get().low().GetSet_LoopTimeRange2(
                self.project.context().to_raw(),
                false,
                is_loop,
                start.as_mut_ptr(),
                end.as_mut_ptr(),
                false,
            );
            (
                Position::from(start.assume_init()),
                Position::from(end.assume_init()),
            )
        }
    }

    pub fn set(&self, start: Position, end: Position) {
        unsafe {
            let is_loop = match self.kind {
                TimeRangeKind::LoopSelection => true,
                TimeRangeKind::TimeSelection => false,
            };
            let (mut start, mut end) =
                (MaybeUninit::new(start.into()), MaybeUninit::new(end.into()));
            Reaper::get().low().GetSet_LoopTimeRange2(
                self.project.context().to_raw(),
                true,
                is_loop,
                start.as_mut_ptr(),
                end.as_mut_ptr(),
                false,
            );
        }
    }

    pub fn get_start(&self) -> Position {
        self.get().0
    }
    pub fn get_end(&self) -> Position {
        self.get().1
    }

    pub fn set_start(&self, start: Position) {
        let end = self.get().1;
        self.set(start, end)
    }
    pub fn set_end(&self, end: Position) {
        let start = self.get().0;
        self.set(start, end)
    }

    pub fn get_length(&self) -> Duration {
        let (start, end) = self.get();
        end.as_duration() - start.as_duration()
    }
    pub fn set_length(&self, length: Duration) {
        let start = self.get().0;
        let end = Position::from(start.as_duration() + length);
        self.set(start, end)
    }

    /// Move selection left or right.
    ///
    /// Returns true if snap is enabled.
    pub fn shift(&self, direction: Direction) -> bool {
        let low = Reaper::get().low();
        unsafe {
            match direction {
                Direction::Right => {
                    low.Loop_OnArrow(self.project.context().to_raw(), 1)
                }
                Direction::Left => {
                    low.Loop_OnArrow(self.project.context().to_raw(), -1)
                }
            }
        }
    }
}

pub struct TimeSignature {
    numerator: u32,
    denominator: u32,
}
impl TimeSignature {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
    pub fn get(&self) -> (u32, u32) {
        (self.numerator, self.denominator)
    }
}

pub trait ProbablyMutable{}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mutable;
impl ProbablyMutable for Mutable {}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Immutable;
impl ProbablyMutable for Immutable {}
