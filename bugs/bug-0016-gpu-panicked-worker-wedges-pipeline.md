# A panicked GPU worker wedges the pipeline forever — `try_recv` folds `Disconnected` into “still running”

**Severity:** medium — one worker panic disables the GPU pipeline until app restart and pins the render loop
**Type:** CODE bug
**Where:** `src/components/image_viewer.rs:1229-1234` (`let Ok(result) = job.handle.try_recv() else { return false; }`); consequences: `src/app_render.rs:114-116` (`is_processing_gpu()` keeps `request_animation_frame` spinning), `src/components/image_viewer.rs:1122-1126` (every future `update_gpu_pipeline` takes the busy branch and stashes into `pending_gpu_params` — no worker ever spawns again)
**Verified:** `check_gpu_processing` read directly; the `else` arm cannot distinguish `TryRecvError::Empty` from `TryRecvError::Disconnected`

## Description

If the rayon worker panics (any wgpu validation or allocation panic not covered by the dimension guard — device loss, out-of-memory on a huge buffer), the result sender is dropped without sending.  `try_recv()` then returns `Err(Disconnected)` forever, which the `let Ok(...) else` collapses into the same “not ready yet” as `Empty`.  `gpu_job` stays `Some` permanently:

- `is_processing_gpu()` is true every frame → the render loop requests animation frames continuously (busy spinning).
- Every subsequent parameter change takes the worker-busy branch, stashing params that will never be picked up.

The pipeline is dead until restart, with no message.  Same two-state-probe-hiding-a-third-state shape as the portfolio’s `positive-evidence-of-absence` rule, applied to a channel.

## Reproduction (Rust test)

```rust
#[test]
fn disconnected_gpu_worker_clears_the_job() {
    // Construct a GpuJob whose sender is dropped without sending (simulated panic).
    let (tx, rx) = std::sync::mpsc::channel();
    drop(tx);
    viewer.gpu_job = Some(GpuJob { handle: rx, cancel: ..., params: ..., frame_idx: None });

    let _ = viewer.check_gpu_processing();
    assert!(viewer.gpu_job.is_none(), "disconnected worker must clear the job");  // FAILS today
    assert!(!viewer.is_processing_gpu());
}
```

## Suggested Fix

Match the error explicitly:

```rust
match job.handle.try_recv() {
    Ok(result) => { /* existing install path */ }
    Err(std::sync::mpsc::TryRecvError::Empty) => return false,
    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
        self.gpu_job = None;
        eprintln!("[GPU] worker terminated abnormally; pipeline reset");
        if let Some(pending) = self.pending_gpu_params.take() { self.update_gpu_pipeline(pending); }
        return false;
    }
}
```

(Together with the error-visibility bug — `gpu-errors-silent-in-release` — the eprintln should ideally be a toast.)

## Why This Fix

Distinguishing `Disconnected` restores the invariant that `gpu_job.is_some()` ⇔ a worker is actually alive; the pending-params re-entry lets the pipeline self-heal on the next parameter change instead of requiring a restart.
