use glint_helper::attribution::{
    ApplicationSource, EvidenceKind, Observation, normalize_label, project,
};

fn row(application: Option<&str>, device: Option<&str>, path: Option<&str>) -> Observation {
    Observation {
        application: application.map(str::to_owned),
        application_source: ApplicationSource::Process,
        device: device.map(str::to_owned),
        device_path: path.map(str::to_owned),
        evidence: EvidenceKind::DeviceAccess,
    }
}

#[test]
fn normalizes_untrusted_labels() {
    assert_eq!(normalize_label("  brave\n"), Some("brave".into()));
    assert_eq!(normalize_label(&"é".repeat(41)), Some("é".repeat(40)));
    assert_eq!(normalize_label(" \t\n"), None);
    let controls = "\0\u{7f}\u{85}\u{061c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}";
    assert_eq!(normalize_label(&format!("a{controls}b")), Some("ab".into()));
    assert_eq!(
        normalize_label(&format!("{}éz", "a".repeat(79))),
        Some("a".repeat(79))
    );
}

#[test]
fn nullable_fields_are_independent_and_labels_are_normalized() {
    let rows = [
        row(None, Some(" Camera\n"), None),
        row(Some(" App "), None, Some("/dev/video2")),
    ];
    let details = project(&rows);
    assert_eq!(details.observations[0], row(None, Some("Camera"), None));
    assert_eq!(
        details.observations[1],
        row(Some("App"), None, Some("/dev/video2"))
    );
    assert!(!details.truncated);
}

#[test]
fn exact_duplicates_only_are_removed() {
    let first = row(Some("App"), Some("Camera"), None);
    let capture = Observation {
        evidence: EvidenceKind::PipewireCapture,
        ..first.clone()
    };
    let reported = Observation {
        application_source: ApplicationSource::PipewireReported,
        ..first.clone()
    };
    let unknown = Observation {
        application_source: ApplicationSource::Unknown,
        ..first.clone()
    };
    let other_device = row(Some("App"), Some("Other"), None);
    let details = project(&[
        first.clone(),
        first,
        capture,
        reported,
        unknown,
        other_device,
    ]);
    assert_eq!(details.observations.len(), 5);
    assert!(!details.truncated);
}

#[test]
fn caps_after_deduplication_and_orders_shuffled_input() {
    let rows: Vec<_> = (0..33)
        .map(|n| row(Some(&format!("App {n:02}")), None, None))
        .collect();
    let reversed: Vec<_> = rows
        .iter()
        .rev()
        .cloned()
        .chain(rows.iter().cloned())
        .collect();
    let details = project(&rows);
    assert_eq!(details, project(&reversed));
    assert_eq!(details.observations, rows[..32]);
    assert!(details.truncated);
    assert!(!project(&rows[..32]).truncated);
}

#[test]
fn rejects_invalid_paths_and_preserves_boundary_paths() {
    for path in [
        "/dev/video",
        "/dev/video-1",
        "/dev/video1/",
        "/dev/video１",
        " /dev/video1",
        "/tmp/video1",
        &format!("/dev/video{}", "1".repeat(55)),
    ] {
        assert_eq!(
            project(&[row(None, None, Some(path))]).observations[0].device_path,
            None
        );
    }
    let valid = format!("/dev/video{}", "1".repeat(54));
    assert_eq!(valid.len(), 64);
    assert_eq!(
        project(&[row(None, None, Some(&valid))]).observations[0].device_path,
        Some(valid)
    );
}

#[test]
fn ordering_uses_path_application_source_and_evidence_with_nulls_first() {
    let rows = [
        row(None, None, Some("/dev/video1")),
        row(Some("Z"), None, None),
        Observation {
            application_source: ApplicationSource::Unknown,
            ..row(None, None, None)
        },
        row(None, None, None),
    ];
    assert_eq!(
        project(&rows).observations,
        vec![
            rows[3].clone(),
            rows[2].clone(),
            rows[1].clone(),
            rows[0].clone()
        ]
    );
}

#[test]
fn equal_primary_keys_have_deterministic_device_ties_and_normalized_duplicates() {
    let base = row(Some("App"), Some("A"), None);
    let rows = [
        Observation {
            device: Some("Z".into()),
            ..base.clone()
        },
        Observation {
            evidence: EvidenceKind::PipewireCapture,
            ..base.clone()
        },
        Observation {
            application_source: ApplicationSource::Unknown,
            ..base.clone()
        },
        Observation {
            application_source: ApplicationSource::PipewireReported,
            ..base.clone()
        },
        base.clone(),
        row(Some(" App\n"), Some(" A "), None),
    ];
    let reverse: Vec<_> = rows.iter().rev().cloned().collect();
    let result = project(&rows);
    assert_eq!(result, project(&reverse));
    assert_eq!(
        result.observations,
        vec![
            rows[3].clone(),
            base,
            rows[0].clone(),
            rows[1].clone(),
            rows[2].clone()
        ]
    );
    assert!(!result.truncated);
}
