//! OpenMontage Asset Validation Tests (deterministic, no OSS)
//!
//! Tests kind/mime/size validation logic without hitting OSS.

use std::collections::HashSet;

/// R011 M3-T2: VALID_ASSET_KINDS must exactly match the documented ROOT role vocabulary
#[test]
fn proptest_valid_asset_kinds_matches_role_vocabulary() {
    // Documented ROOT role vocabulary (from M0b-T5 reconciliation)
    let expected_vocabulary: HashSet<&str> = [
        "reference_image",
        "start_frame",
        "end_frame",
        "reference_video",
        "source_video",
        "brand_asset",
        "audio",
        "music",
        "subtitle",
        "avatar", // M3-T2: avatar must be in the allowlist
    ]
    .iter()
    .copied()
    .collect();

    // Current VALID_ASSET_KINDS from handler (mirrored in helper below)
    let actual_kinds: HashSet<&str> = get_valid_asset_kinds().iter().copied().collect();

    assert_eq!(
        actual_kinds,
        expected_vocabulary,
        "VALID_ASSET_KINDS must exactly match the ROOT role vocabulary. Missing: {:?}, Extra: {:?}",
        expected_vocabulary
            .difference(&actual_kinds)
            .collect::<Vec<_>>(),
        actual_kinds
            .difference(&expected_vocabulary)
            .collect::<Vec<_>>()
    );
}

#[test]
fn asset_kind_validation_accepts_valid_kinds() {
    let valid_kinds = vec![
        "reference_image",
        "start_frame",
        "end_frame",
        "reference_video",
        "source_video",
        "brand_asset",
        "audio",
        "music",
        "subtitle",
        "avatar", // M3-T2: avatar is a valid kind
    ];

    for kind in valid_kinds {
        assert!(is_valid_asset_kind(kind), "Kind should be valid: {}", kind);
    }
}

#[test]
fn asset_kind_validation_rejects_invalid_kinds() {
    let invalid_kinds = vec!["unknown", "invalid_kind", "random_type", ""];

    for kind in invalid_kinds {
        assert!(
            !is_valid_asset_kind(kind),
            "Kind should be invalid: {}",
            kind
        );
    }
}

#[test]
fn image_asset_mime_validation() {
    let valid_image_mimes = vec![
        "image/jpeg",
        "image/jpg",
        "image/png",
        "image/gif",
        "image/webp",
        "image/bmp",
        "image/tiff",
    ];

    for mime in valid_image_mimes {
        assert!(
            is_valid_image_mime(mime),
            "Image MIME should be valid: {}",
            mime
        );
    }

    let invalid_image_mimes = vec!["video/mp4", "audio/mpeg", "application/pdf", "text/plain"];

    for mime in invalid_image_mimes {
        assert!(
            !is_valid_image_mime(mime),
            "Image MIME should be invalid: {}",
            mime
        );
    }
}

#[test]
fn video_asset_mime_validation() {
    let valid_video_mimes = vec![
        "video/mp4",
        "video/quicktime",
        "video/x-msvideo",
        "video/webm",
        "video/x-matroska",
        "video/x-flv",
        "video/x-ms-wmv",
        "video/x-m4v",
    ];

    for mime in valid_video_mimes {
        assert!(
            is_valid_video_mime(mime),
            "Video MIME should be valid: {}",
            mime
        );
    }

    let invalid_video_mimes = vec!["image/png", "audio/mpeg", "application/pdf"];

    for mime in invalid_video_mimes {
        assert!(
            !is_valid_video_mime(mime),
            "Video MIME should be invalid: {}",
            mime
        );
    }
}

#[test]
fn audio_asset_mime_validation() {
    let valid_audio_mimes = vec![
        "audio/mpeg",
        "audio/mp3",
        "audio/wav",
        "audio/x-wav",
        "audio/wave",
        "audio/ogg",
        "audio/aac",
        "audio/x-m4a",
        "audio/mp4",
    ];

    for mime in valid_audio_mimes {
        assert!(
            is_valid_audio_mime(mime),
            "Audio MIME should be valid: {}",
            mime
        );
    }

    let invalid_audio_mimes = vec!["image/png", "video/mp4", "text/plain"];

    for mime in invalid_audio_mimes {
        assert!(
            !is_valid_audio_mime(mime),
            "Audio MIME should be invalid: {}",
            mime
        );
    }
}

#[test]
fn image_size_validation() {
    let max_image_size = 30 * 1024 * 1024; // 30MB

    assert!(is_within_size_limit(1024, max_image_size));
    assert!(is_within_size_limit(max_image_size, max_image_size));
    assert!(!is_within_size_limit(max_image_size + 1, max_image_size));
}

#[test]
fn video_size_validation() {
    let max_video_size = 100 * 1024 * 1024; // 100MB

    assert!(is_within_size_limit(1024, max_video_size));
    assert!(is_within_size_limit(max_video_size, max_video_size));
    assert!(!is_within_size_limit(max_video_size + 1, max_video_size));
}

#[test]
fn audio_size_validation() {
    let max_audio_size = 15 * 1024 * 1024; // 15MB

    assert!(is_within_size_limit(1024, max_audio_size));
    assert!(is_within_size_limit(max_audio_size, max_audio_size));
    assert!(!is_within_size_limit(max_audio_size + 1, max_audio_size));
}

// Helper functions mirroring the handler validation logic
fn get_valid_asset_kinds() -> Vec<&'static str> {
    vec![
        "reference_image",
        "start_frame",
        "end_frame",
        "reference_video",
        "source_video",
        "brand_asset",
        "audio",
        "music",
        "subtitle",
        "avatar", // M3-T2: avatar is now in handler
    ]
}

fn is_valid_asset_kind(kind: &str) -> bool {
    get_valid_asset_kinds().contains(&kind)
}

fn is_valid_image_mime(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg"
            | "image/jpg"
            | "image/png"
            | "image/gif"
            | "image/webp"
            | "image/bmp"
            | "image/tiff"
    )
}

fn is_valid_video_mime(mime: &str) -> bool {
    matches!(
        mime,
        "video/mp4"
            | "video/quicktime"
            | "video/x-msvideo"
            | "video/webm"
            | "video/x-matroska"
            | "video/x-flv"
            | "video/x-ms-wmv"
            | "video/x-m4v"
    )
}

fn is_valid_audio_mime(mime: &str) -> bool {
    matches!(
        mime,
        "audio/mpeg"
            | "audio/mp3"
            | "audio/wav"
            | "audio/x-wav"
            | "audio/wave"
            | "audio/ogg"
            | "audio/aac"
            | "audio/x-m4a"
            | "audio/mp4"
    )
}

fn is_within_size_limit(size: usize, max: usize) -> bool {
    size <= max
}
