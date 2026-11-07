use super::*;

fn open(r: &mut Registry) -> u64 {
    r.open(Version::default()).unwrap()
}

#[test]
fn negotiation_rejects_without_allocating_a_handle() {
    let mut r = Registry::default();
    for version in [
        Version {
            major: 2,
            ..Version::default()
        },
        Version {
            minor: 1,
            ..Version::default()
        },
    ] {
        assert_eq!(r.open(version), Err(Status::VERSION));
    }
    for version in [
        Version {
            struct_size: 0,
            ..Version::default()
        },
        Version {
            struct_size: 24,
            ..Version::default()
        },
        Version {
            reserved: 1,
            ..Version::default()
        },
    ] {
        assert_eq!(r.open(version), Err(Status::ARGUMENT));
    }
    assert_eq!(open(&mut r), 1);
}

#[test]
fn allocation_is_initialized_and_all_or_nothing_at_boundaries() {
    let mut r = Registry::default();
    let c = open(&mut r);
    let h = r.allocate(c, 4).unwrap();
    let mut out = [255; 4];
    r.read(c, h, 0, &mut out).unwrap();
    assert_eq!(out, [0; 4]);
    r.write(c, h, 1, &[7, 8]).unwrap();
    for offset in [3, 4, u64::MAX] {
        assert_eq!(r.write(c, h, offset, &[9, 9]), Err(Status::ARGUMENT));
    }
    r.read(c, h, 0, &mut out).unwrap();
    assert_eq!(out, [0, 7, 8, 0]);
    let mut unchanged = [99; 2];
    assert_eq!(r.read(c, h, 3, &mut unchanged), Err(Status::ARGUMENT));
    assert_eq!(unchanged, [99; 2]);
    r.write(c, h, 4, &[]).unwrap();
    r.read(c, h, 4, &mut []).unwrap();
    assert_eq!(r.write(c, h, 5, &[]), Err(Status::ARGUMENT));
}

#[test]
fn wrong_kind_owner_zero_and_double_free_never_remove_live_buffer() {
    let mut r = Registry::default();
    let a = open(&mut r);
    let b = open(&mut r);
    let h = r.allocate(a, 8).unwrap();
    for (owner, handle) in [(b, h), (a, a), (a, b), (a, 0), (0, h)] {
        assert_eq!(r.free(owner, handle), Err(Status::HANDLE));
        assert_eq!(r.length(owner, handle), Err(Status::HANDLE));
    }
    assert_eq!(r.close(h), Err(Status::HANDLE));
    assert_eq!(r.allocated_bytes(), 8);
    assert_eq!(r.length(a, h), Ok(8));
    r.free(a, h).unwrap();
    assert_eq!(r.free(a, h), Err(Status::HANDLE));
    assert_eq!(r.allocated_bytes(), 0);
}

#[test]
fn close_revokes_all_owned_handles_without_touching_other_context() {
    let mut r = Registry::default();
    let a = open(&mut r);
    let b = open(&mut r);
    let h = r.allocate(a, 5).unwrap();
    r.allocate(a, 7).unwrap();
    let other = r.allocate(b, 3).unwrap();
    r.write(a, h, 0, b"secret".get(..5).unwrap()).unwrap();
    r.close(a).unwrap();
    assert_eq!(r.allocated_bytes(), 3);
    assert_eq!(r.close(a), Err(Status::HANDLE));
    assert_eq!(r.read(a, h, 0, &mut [0]), Err(Status::HANDLE));
    assert_eq!(r.allocate(a, 1), Err(Status::HANDLE));
    assert_eq!(r.length(b, other), Ok(3));
    let new = open(&mut r);
    assert!(new > other);
    assert_eq!(r.length(new, h), Err(Status::HANDLE));
}

#[test]
fn handle_space_never_wraps_or_reuses_after_exhaustion() {
    let mut r = Registry {
        next: u64::MAX - 1,
        ..Registry::default()
    };
    let c = open(&mut r);
    let h = r.allocate(c, 1).unwrap();
    assert_eq!(h, u64::MAX);
    assert_eq!(r.allocate(c, 1), Err(Status::CAPACITY));
    assert_eq!(r.allocated_bytes(), 1);
    r.free(c, h).unwrap();
    r.close(c).unwrap();
    assert_eq!(r.open(Version::default()), Err(Status::CAPACITY));
    assert_eq!(r.allocated_bytes(), 0);
}

#[test]
fn context_and_buffer_count_limits_include_zero_length_allocations() {
    let mut r = Registry::default();
    let contexts: Vec<_> = (0..MAX_CONTEXTS).map(|_| open(&mut r)).collect();
    assert_eq!(r.open(Version::default()), Err(Status::CAPACITY));
    let c = contexts[0];
    let buffers: Vec<_> = (0..MAX_BUFFERS)
        .map(|_| r.allocate(c, 0).unwrap())
        .collect();
    assert_eq!(r.allocate(c, 0), Err(Status::CAPACITY));
    assert_eq!(r.allocated_bytes(), 0);
    r.free(c, buffers[0]).unwrap();
    assert!(r.allocate(c, 0).is_ok());
    r.close(c).unwrap();
    assert!(r.open(Version::default()).is_ok());
}

#[test]
fn live_byte_budget_is_global_and_released_on_close() {
    let mut r = Registry::default();
    let a = open(&mut r);
    let b = open(&mut r);
    assert_eq!(r.allocate(a, MAX_BUFFER_BYTES + 1), Err(Status::ARGUMENT));
    for _ in 0..MAX_TOTAL_BYTES / MAX_BUFFER_BYTES {
        r.allocate(a, MAX_BUFFER_BYTES).unwrap();
    }
    assert_eq!(r.allocated_bytes(), MAX_TOTAL_BYTES);
    assert_eq!(r.allocate(b, 1), Err(Status::CAPACITY));
    let empty = r.allocate(b, 0).unwrap();
    r.close(a).unwrap();
    assert_eq!(r.allocated_bytes(), 0);
    assert_eq!(r.length(b, empty), Ok(0));
    assert!(r.allocate(b, MAX_BUFFER_BYTES).is_ok());
}

#[test]
fn every_stale_handle_remains_invalid_across_many_lifecycles() {
    let mut r = Registry::default();
    let mut stale = Vec::new();
    for _ in 0..128 {
        let c = open(&mut r);
        let h = r.allocate(c, 1).unwrap();
        for &old in &stale {
            assert_eq!(r.length(c, old), Err(Status::HANDLE));
        }
        stale.push(h);
        r.close(c).unwrap();
    }
}

#[test]
fn adapter_liveness_and_borrowed_views_share_exact_registry_ownership() {
    let mut r = Registry::default();
    let a = open(&mut r);
    let b = open(&mut r);
    let h = r.allocate(a, 4).unwrap();
    r.write(a, h, 0, &[0, 255, 42, 0]).unwrap();
    assert_eq!(r.validate_context(a), Ok(()));
    assert_eq!(r.validate_context(h), Err(Status::HANDLE));
    assert_eq!(r.validate_context(0), Err(Status::HANDLE));
    assert_eq!(r.buffer_bytes(a, h), Ok([0, 255, 42, 0].as_slice()));
    assert_eq!(r.buffer_bytes(b, h), Err(Status::HANDLE));
    assert_eq!(r.buffer_bytes(a, a), Err(Status::HANDLE));
    r.close(a).unwrap();
    assert_eq!(r.validate_context(a), Err(Status::HANDLE));
    assert_eq!(r.buffer_bytes(a, h), Err(Status::HANDLE));
    assert_eq!(r.buffer_bytes(b, h), Err(Status::HANDLE));
}
