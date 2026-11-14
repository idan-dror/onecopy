//! Deterministic per-file fault plan (spec §3.3).
//!
//! The provider asks the plan what to do for each `IStream::Read`. The plan is pure: it does
//! not sleep, kill or touch COM. The provider carries out the returned action. This keeps the
//! fault semantics unit-testable without Windows.
//!
//! Trigger rule: a fault "at offset X" applies to the first read that **reaches** X. A read
//! starting at `p` that would serve `n` bytes reaches X when `X < p + max(n, 1)`. So:
//! - a zero-byte end-of-stream read at `p == X` reaches X;
//! - `X = size - 1` hits the read that would deliver the final byte;
//! - `X = size` hits the end-of-stream read that follows the final data read.
//!
//! Faults with `once = true` fire only once per file per provider process. The caller claims
//! them atomically, so concurrent streams cannot both fire them.

use serde::{Deserialize, Serialize};

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Fault {
    /// Adds a fixed delay to every read.
    Latency { ms: u64 },
    /// Caps throughput from `from_offset` onwards.
    Throttle {
        mib_per_s: f64,
        #[serde(default)]
        from_offset: u64,
    },
    /// Blocks the read that reaches `at_offset` for `duration_ms`, then continues.
    Stall {
        at_offset: u64,
        duration_ms: u64,
        #[serde(default = "yes")]
        once: bool,
    },
    /// The read that reaches `at_offset` returns `hresult` (a failure code) and no bytes.
    Error {
        at_offset: u64,
        hresult: u32,
        #[serde(default)]
        once: bool,
    },
    /// The stream ends at `at_offset`, whatever the advertised size.
    PrematureEof { at_offset: u64 },
    /// The stream delivers `extra_bytes` beyond the file size.
    Overrun { extra_bytes: u64 },
    /// No read returns more than `max_bytes` (S_OK short reads, not end of stream).
    ShortReads { max_bytes: u32 },
    /// The provider terminates itself during the read that reaches `at_offset`.
    SelfKill { at_offset: u64 },
    /// No effect; logs an event the harness can trigger on (once).
    Mark { at_offset: u64, label: String },
}

impl Fault {
    pub fn is_once(&self) -> bool {
        match self {
            Fault::Stall { once, .. } | Fault::Error { once, .. } => *once,
            Fault::SelfKill { .. } | Fault::Mark { .. } => true,
            _ => false,
        }
    }

    fn trigger_offset(&self) -> Option<u64> {
        match self {
            Fault::Stall { at_offset, .. }
            | Fault::Error { at_offset, .. }
            | Fault::SelfKill { at_offset }
            | Fault::Mark { at_offset, .. } => Some(*at_offset),
            _ => None,
        }
    }
}

/// What one read must do.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadAction {
    /// Fixed latency to add (ms).
    pub latency_ms: u64,
    /// Minimum duration of this read to respect a throttle (ms).
    pub throttle_ms: f64,
    /// Stall to perform before the outcome: (fault index, duration ms).
    pub stall: Option<(usize, u64)>,
    /// Mark faults reached by this read.
    pub marks: Vec<usize>,
    pub outcome: ReadOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadOutcome {
    /// Serve `bytes`. `end_of_stream` is true when fewer bytes than requested are returned
    /// because the stream ended (the provider then returns S_FALSE).
    Serve { bytes: u32, end_of_stream: bool },
    /// Return `hresult` and no bytes.
    Fail { fault: usize, hresult: u32 },
    /// Terminate the provider process.
    Kill { fault: usize },
}

/// The fault plan of one file.
#[derive(Debug, Clone)]
pub struct FilePlan {
    pub size: u64,
    pub faults: Vec<Fault>,
}

impl FilePlan {
    pub fn new(size: u64, faults: Vec<Fault>) -> Self {
        Self { size, faults }
    }

    /// Bytes the stream delivers before reporting end of stream.
    pub fn stream_len(&self) -> u64 {
        let mut len = self.size;
        for f in &self.faults {
            if let Fault::Overrun { extra_bytes } = f {
                len = self.size + extra_bytes;
            }
        }
        for f in &self.faults {
            if let Fault::PrematureEof { at_offset } = f {
                len = len.min(*at_offset);
            }
        }
        len
    }

    /// Plans one read at `position` for `requested` bytes. `claim(i)` must atomically claim a
    /// once-fault `i` and return true only for the first caller.
    pub fn plan_read(&self, position: u64, requested: u32, claim: &dyn Fn(usize) -> bool) -> ReadAction {
        let len = self.stream_len();
        let mut n = (requested as u64).min(len.saturating_sub(position));
        for f in &self.faults {
            if let Fault::ShortReads { max_bytes } = f {
                n = n.min(*max_bytes as u64);
            }
        }
        let reach = position + n.max(1);
        let mut latency_ms = 0;
        let mut rate: Option<f64> = None;
        let mut stall = None;
        let mut marks = Vec::new();
        let mut fail = None;
        let mut kill = None;
        for (i, f) in self.faults.iter().enumerate() {
            match f {
                Fault::Latency { ms } => latency_ms += ms,
                Fault::Throttle { mib_per_s, from_offset } if position >= *from_offset => {
                    rate = Some(rate.map_or(*mib_per_s, |r: f64| r.min(*mib_per_s)));
                }
                _ => {}
            }
            let Some(x) = f.trigger_offset() else { continue };
            if x >= reach {
                continue;
            }
            if f.is_once() && !claim(i) {
                continue;
            }
            match f {
                Fault::Stall { duration_ms, .. } if stall.is_none() => stall = Some((i, *duration_ms)),
                Fault::Error { hresult, .. } if fail.is_none() => fail = Some((i, *hresult)),
                Fault::SelfKill { .. } if kill.is_none() => kill = Some(i),
                Fault::Mark { .. } => marks.push(i),
                _ => {}
            }
        }
        let outcome = if let Some(fault) = kill {
            ReadOutcome::Kill { fault }
        } else if let Some((fault, hresult)) = fail {
            ReadOutcome::Fail { fault, hresult }
        } else {
            let bytes = n as u32;
            ReadOutcome::Serve { bytes, end_of_stream: bytes < requested && position + n >= len }
        };
        let served = match outcome {
            ReadOutcome::Serve { bytes, .. } => bytes as f64,
            _ => 0.0,
        };
        let throttle_ms = rate.map_or(0.0, |r| served / (r * 1_048_576.0) * 1000.0);
        ReadAction { latency_ms, throttle_ms, stall, marks, outcome }
    }

    /// Checks that the plan is internally consistent and only returns failure HRESULTs.
    pub fn validate(&self) -> Result<(), String> {
        let mut eof = 0;
        let mut overrun = 0;
        for f in &self.faults {
            match f {
                Fault::PrematureEof { at_offset } => {
                    eof += 1;
                    if *at_offset > self.size {
                        return Err("premature_eof beyond the file size".into());
                    }
                }
                Fault::Overrun { .. } => overrun += 1,
                // HRESULT discipline (W-01): an injected error must be a failure code, never a
                // success code such as S_FALSE that a caller could mistake for success.
                Fault::Error { hresult, .. } if hresult & 0x8000_0000 == 0 => {
                    return Err(format!("injected hresult {hresult:#010x} is not a failure code"));
                }
                Fault::Throttle { mib_per_s, .. } if mib_per_s.is_nan() || *mib_per_s <= 0.0 => return Err("throttle must be > 0".into()),
                Fault::Stall { duration_ms, .. } if *duration_ms > 600_000 => return Err("stall longer than 10 minutes".into()),
                Fault::Latency { ms } if *ms > 60_000 => return Err("latency longer than 60 s".into()),
                Fault::ShortReads { max_bytes: 0 } => return Err("short_reads max_bytes must be > 0".into()),
                _ => {}
            }
        }
        if eof > 1 || overrun > 1 || (eof > 0 && overrun > 0) {
            return Err("at most one of premature_eof / overrun".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const MIB: u64 = 1 << 20;
    const CHUNK: u32 = 256 * 1024;

    /// Simulates a sequential reader; returns (offset, outcome) per read until EOF, fail or kill.
    fn run(plan: &FilePlan, chunk: u32) -> Vec<(u64, ReadOutcome)> {
        let claimed = RefCell::new(vec![false; plan.faults.len()]);
        let claim = |i: usize| !std::mem::replace(&mut claimed.borrow_mut()[i], true);
        let mut pos = 0;
        let mut out = Vec::new();
        for _ in 0..100_000 {
            let a = plan.plan_read(pos, chunk, &claim);
            out.push((pos, a.outcome));
            match a.outcome {
                ReadOutcome::Serve { bytes, end_of_stream } => {
                    pos += bytes as u64;
                    if end_of_stream || bytes == 0 {
                        break;
                    }
                }
                _ => break,
            }
        }
        out
    }

    #[test]
    fn clean_stream_ends_with_short_then_zero_read() {
        let plan = FilePlan::new(MIB, vec![]);
        let reads = run(&plan, CHUNK);
        assert_eq!(reads.len(), 5); // 4 full chunks, then a zero-byte EOF read
        assert_eq!(reads[4], (MIB, ReadOutcome::Serve { bytes: 0, end_of_stream: true }));
    }

    #[test]
    fn premature_eof_ends_stream_early() {
        let plan = FilePlan::new(MIB, vec![Fault::PrematureEof { at_offset: MIB / 2 + 10 }]);
        assert_eq!(plan.stream_len(), MIB / 2 + 10);
        let reads = run(&plan, CHUNK);
        let total: u64 = reads.iter().map(|(_, o)| if let ReadOutcome::Serve { bytes, .. } = o { *bytes as u64 } else { 0 }).sum();
        assert_eq!(total, MIB / 2 + 10);
        assert!(matches!(reads.last().unwrap().1, ReadOutcome::Serve { end_of_stream: true, .. }));
    }

    #[test]
    fn overrun_delivers_extra_bytes() {
        let plan = FilePlan::new(MIB, vec![Fault::Overrun { extra_bytes: 1000 }]);
        assert_eq!(plan.stream_len(), MIB + 1000);
    }

    #[test]
    fn error_hits_the_read_that_reaches_the_offset() {
        let plan = FilePlan::new(MIB, vec![Fault::Error { at_offset: MIB / 2, hresult: 0x8000_4005, once: false }]);
        let reads = run(&plan, CHUNK);
        assert_eq!(reads.last().unwrap(), &(MIB / 2, ReadOutcome::Fail { fault: 0, hresult: 0x8000_4005 }));
        // An offset inside a chunk fails that whole chunk: bytes before it in the chunk are not served.
        let plan = FilePlan::new(MIB, vec![Fault::Error { at_offset: MIB / 2 + 5, hresult: 0x8000_4005, once: false }]);
        assert_eq!(run(&plan, CHUNK).last().unwrap().0, MIB / 2);
    }

    #[test]
    fn final_byte_and_eof_read_offsets() {
        // size - 1: the read delivering the last data fails.
        let plan = FilePlan::new(MIB, vec![Fault::Error { at_offset: MIB - 1, hresult: 0x8003_001E, once: false }]);
        assert_eq!(run(&plan, CHUNK).last().unwrap().0, MIB - CHUNK as u64);
        // size: every data byte is served, then the end-of-stream read fails.
        let plan = FilePlan::new(MIB, vec![Fault::Error { at_offset: MIB, hresult: 0x8003_001E, once: false }]);
        assert_eq!(run(&plan, CHUNK).last().unwrap(), &(MIB, ReadOutcome::Fail { fault: 0, hresult: 0x8003_001E }));
    }

    #[test]
    fn once_faults_fire_once_per_plan_claim() {
        let plan = FilePlan::new(MIB, vec![Fault::Error { at_offset: 0, hresult: 0x8000_4005, once: true }]);
        let claimed = RefCell::new(vec![false]);
        let claim = |i: usize| !std::mem::replace(&mut claimed.borrow_mut()[i], true);
        assert!(matches!(plan.plan_read(0, CHUNK, &claim).outcome, ReadOutcome::Fail { .. }));
        // A retry (new stream from 0) succeeds.
        assert!(matches!(plan.plan_read(0, CHUNK, &claim).outcome, ReadOutcome::Serve { bytes: CHUNK, .. }));
    }

    #[test]
    fn stall_then_serve_and_kill_precedence() {
        let plan = FilePlan::new(MIB, vec![
            Fault::Stall { at_offset: 0, duration_ms: 500, once: true },
            Fault::SelfKill { at_offset: 0 },
        ]);
        let a = plan.plan_read(0, CHUNK, &|_| true);
        assert_eq!(a.stall, Some((0, 500)));
        assert_eq!(a.outcome, ReadOutcome::Kill { fault: 1 });
    }

    #[test]
    fn short_reads_are_not_end_of_stream() {
        let plan = FilePlan::new(MIB, vec![Fault::ShortReads { max_bytes: 4096 }]);
        let a = plan.plan_read(0, CHUNK, &|_| true);
        assert_eq!(a.outcome, ReadOutcome::Serve { bytes: 4096, end_of_stream: false });
        let a = plan.plan_read(MIB - 100, CHUNK, &|_| true);
        assert_eq!(a.outcome, ReadOutcome::Serve { bytes: 100, end_of_stream: true });
    }

    #[test]
    fn throttle_applies_from_offset() {
        let plan = FilePlan::new(MIB, vec![Fault::Throttle { mib_per_s: 1.0, from_offset: MIB / 2 }]);
        assert_eq!(plan.plan_read(0, CHUNK, &|_| true).throttle_ms, 0.0);
        let t = plan.plan_read(MIB / 2, CHUNK, &|_| true).throttle_ms;
        assert!((t - 250.0).abs() < 1e-6, "{t}");
    }

    #[test]
    fn marks_fire_once() {
        let plan = FilePlan::new(MIB, vec![Fault::Mark { at_offset: 10, label: "quarter".into() }]);
        let claimed = RefCell::new(vec![false]);
        let claim = |i: usize| !std::mem::replace(&mut claimed.borrow_mut()[i], true);
        assert_eq!(plan.plan_read(0, CHUNK, &claim).marks, vec![0]);
        assert!(plan.plan_read(CHUNK as u64, CHUNK, &claim).marks.is_empty());
    }

    #[test]
    fn validation_rejects_success_hresults_and_conflicts() {
        assert!(FilePlan::new(MIB, vec![Fault::Error { at_offset: 0, hresult: 1, once: false }]).validate().is_err());
        assert!(FilePlan::new(MIB, vec![Fault::Error { at_offset: 0, hresult: 0, once: false }]).validate().is_err());
        assert!(FilePlan::new(MIB, vec![Fault::PrematureEof { at_offset: 1 }, Fault::Overrun { extra_bytes: 1 }]).validate().is_err());
        assert!(FilePlan::new(MIB, vec![Fault::PrematureEof { at_offset: MIB + 1 }]).validate().is_err());
        assert!(FilePlan::new(MIB, vec![Fault::Error { at_offset: 0, hresult: 0x8000_4005, once: false }]).validate().is_ok());
    }
}
