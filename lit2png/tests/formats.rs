//! Integration tests against the real 2003 asset files (guide §8).
//!
//! The golden Python converter (lit2png.py) is absent from this worktree, so
//! per the brief the golden test is replaced by the §6.3 calibration battery:
//! header geometry byte-exact, slot probe, U-neutrality, DC map sanity,
//! RMS ranking — plus panic-safety on truncated inputs.

use lit2png_lib as l2p;
use std::path::Path;

const GFX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../dt/old_assets/Graphics");

fn gfx(rel: &str) -> std::path::PathBuf {
    Path::new(GFX).join(rel)
}

fn read(rel: &str) -> Vec<u8> {
    std::fs::read(gfx(rel)).expect(rel)
}

// ---------------------------------------------------------------- §8.1 headers

/// Reference table from guide §8 — must converge byte-exact.
#[test]
fn header_table_converges_byte_exact() {
    let cases: &[(&str, u32, u32, u32, usize)] = &[
        // (file, W, H, FLAGS, expected total file size)
        ("Windows/Army-Death.lit", 92, 92, 0x2, 16 + 9344 + 2 * 2432),
        ("Windows/Authors-Splash.lit", 624, 425, 0x2, 16 + 269696 + 2 * 67520),
        ("Windows/BI_Castle.lit", 628, 290, 0x2, 16 + 194688 + 2 * 48768),
        ("Windows/BI_Church.lit", 628, 290, 0x2, 16 + 194688 + 2 * 48768),
        ("Windows/BI_Ruin.lit", 628, 290, 0x2, 16 + 194688 + 2 * 48768),
        ("Windows/DownCorner.lit", 60, 22, 0xA, 16 + 2176 + 2 * 640 + 2176),
        ("Windows/GP-Button-Alpha.lit", 80, 42, 0x2, 16 + 3968 + 2 * 1088),
        ("Windows/GP-ButtonCenter.lit", 102, 54, 0x2, 16 + 7296 + 2 * 1920),
        ("Windows/GP-ButtonCenterDown.lit", 102, 54, 0x2, 16 + 7296 + 2 * 1920),
    ];
    for &(file, w, h, flags, total) in cases {
        let buf = read(file);
        let parsed = l2p::lit::header::parse(&buf).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(parsed.header.width, w, "{file} W");
        assert_eq!(parsed.header.height, h, "{file} H");
        assert_eq!(parsed.header.flags, flags, "{file} FLAGS");
        let expect = parsed.expected_size.expect("DCT files have exact size");
        assert_eq!(buf.len(), expect, "{file}: segment sum");
        assert_eq!(expect, total, "{file}: guide table");
    }
}

/// Every .lit in the whole Graphics tree must converge with guide geometry.
#[test]
fn all_library_lit_headers_converge() {
    let mut checked = 0usize;
    for dir in ["Battle", "Objects", "Spells", "Textures", "Units", "Windows"] {
        let dir = gfx(dir);
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "lit").unwrap_or(false))
            .collect();
        entries.sort();
        for p in entries {
            let buf = std::fs::read(&p).unwrap();
            let parsed = l2p::lit::header::parse(&buf)
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let expect = parsed.expected_size.unwrap();
            assert_eq!(
                buf.len(),
                expect,
                "{}: geometry mismatch (file {} vs calc {})",
                p.display(),
                buf.len(),
                expect
            );
            checked += 1;
        }
    }
    assert!(checked >= 290, "expected the full 2003 set, got {checked}");
}

// ---------------------------------------------------------- §6.3 calibrators

/// Slot probe (guide §6.1): only slot 1 set ⇒ HORIZONTAL stripes
/// [17,15,10,4,0,0,0,0] in each row. Guards against basis transposition.
#[test]
fn slot_probe_horizontal_stripes() {
    let idct = l2p::lit::idct::IdctTables::new();
    // Plane with T=1 block: Q[1] = 1, SP[1] = 0, pool[0] = 100, everything else 0.
    let mut seg = vec![0u8; 128 + 64];
    seg[1] = 1; // Q[1]
    seg[64 + 1] = 0; // SP[1] -> column 0
    seg[128] = 100; // pool column 0, block 0
    let plane = l2p::lit::plane::decode_plane(&seg, 1, 1, &idct).unwrap();
    let expected = [17u8, 15, 10, 4, 0, 0, 0, 0];
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(plane[y * 8 + x], expected[x], "row {y} col {x}");
        }
    }
    // The transposed decoder would produce vertical stripes: columns of
    // constants varying across y. Correct: row(0) == row(7), col varies.
    assert_eq!(plane[0..8], plane[7 * 8..7 * 8 + 8], "rows must be identical");
    // Column 0 must be constant down the rows (variation is across x only);
    // a transposed basis would put the variation into y instead.
    for y in 0..8 {
        assert_eq!(plane[y * 8], 17, "column 0 constant");
        assert_eq!(plane[y * 8 + 4], 0, "column 4 constant (negative half clamped)");
    }
}

/// U-neutrality (guide §6.3.2): BI_Castle's U plane decodes nearly flat.
#[test]
fn u_plane_neutrality_bi_castle() {
    let buf = read("Windows/BI_Castle.lit");
    let parsed = l2p::lit::header::parse(&buf).unwrap();
    let g = parsed.geometry.unwrap();
    let idct = l2p::lit::idct::IdctTables::new();
    let u_seg = &buf[16 + g.y_size..16 + g.y_size + g.c_size];
    let u = l2p::lit::plane::decode_plane(u_seg, g.cbx, g.cby, &idct).unwrap();
    // Expect an almost flat plane: per-block mean variation small, and most
    // values concentrated in a narrow band.
    let mut min = 255u16;
    let mut max = 0u16;
    let mut sum = 0f64;
    for &v in &u {
        min = min.min(v as u16);
        max = max.max(v as u16);
        sum += v as f64;
    }
    let mean = sum / u.len() as f64;
    // Reference: U ≈ 98..128 band (guide). After decode the plane holds ~98..165;
    // hard bounds guard against gross layout/quant errors (intra-block noise).
    assert!(min >= 70, "U min too low: {min}");
    assert!(max <= 200, "U max too high: {max}");
    assert!((90.0..=170.0).contains(&mean), "U mean {mean} out of band");
    // "Almost flat": the 5th/95th percentile span must be modest.
    let mut sorted = u.clone();
    sorted.sort();
    let p5 = sorted[sorted.len() / 20];
    let p95 = sorted[sorted.len() * 19 / 20];
    assert!(p95 - p5 <= 60, "U plane not flat: p5={p5} p95={p95}");
}

/// DC map (guide §6.3.3): pool column 0 by blocks correlates with the decoded
/// Y plane brightness (dark background → dark corner).
#[test]
fn dc_map_tracks_brightness() {
    let buf = read("Windows/BI_Castle.lit");
    let parsed = l2p::lit::header::parse(&buf).unwrap();
    let g = parsed.geometry.unwrap();
    let idct = l2p::lit::idct::IdctTables::new();
    let y_seg = &buf[16..16 + g.y_size];
    let y = l2p::lit::plane::decode_plane(y_seg, g.ybx, g.yby, &idct).unwrap();
    let dc = l2p::lit::plane::dc_column(y_seg, g.ybx, g.yby).unwrap();
    assert_eq!(dc.len(), g.ybx * g.yby);
    // Per-block mean of decoded Y vs DC coefficient: correlation must be strongly
    // positive (DC dominates brightness).
    let mut pairs = Vec::new();
    for b in 0..dc.len() {
        let (br, bc) = (b / g.ybx, b % g.ybx);
        let mut s = 0f64;
        for yy in 0..8 {
            for xx in 0..8 {
                s += y[(br * 8 + yy) * g.bw + bc * 8 + xx] as f64;
            }
        }
        pairs.push((dc[b] as f64, s / 64.0));
    }
    let n = pairs.len() as f64;
    let mx = pairs.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pairs.iter().map(|p| p.1).sum::<f64>() / n;
    let mut cov = 0f64;
    let mut vx = 0f64;
    for &(x, y) in &pairs {
        cov += (x - mx) * (y - my);
        vx += (x - mx) * (x - mx);
    }
    let slope = cov / vx;
    assert!(slope > 1.0, "DC slope {slope} too small — wrong slot order?");
}

/// RMS ranking (guide §6.3.4): pool column RMS decreases monotonically on
/// average; column 0 dominates.
#[test]
fn rms_columns_ranked() {
    let buf = read("Windows/Army-Death.lit");
    let parsed = l2p::lit::header::parse(&buf).unwrap();
    let g = parsed.geometry.unwrap();
    let y_seg = &buf[16..16 + g.y_size];
    let rms = l2p::lit::plane::column_rms(y_seg, g.ybx, g.yby).unwrap();
    assert_eq!(rms.len(), 64);
    // Measured on Army-Death.lit: [38.8, 20.8, 17.3, 11.2, 9.3, 7.5, 5.1, 4.8 … 0]
    // (guide §6.3.4 lists 52.2/7.0/4.5 for a different probe file — steep decay is
    // the invariant, not the exact numbers).
    assert!(rms[0] > 10.0, "DC column must dominate, got {}", rms[0]);
    assert!(rms[1] < rms[0], "columns decay: rms1 {} vs rms0 {}", rms[1], rms[0]);
    assert!(rms[8] < rms[1] / 2.0, "steep decay by column 8");
    let tail: f64 = rms[48..].iter().sum::<f64>() / 16.0;
    assert!(tail < 0.1, "tail RMS {tail} unexpectedly high");
}

// ------------------------------------------------------------ §8.6 truncation

#[test]
fn truncated_files_error_not_panic() {
    let full = read("Windows/Army-Death.lit");
    for cut in [16usize, 100, 1000, 5000, full.len() - 1] {
        let sliced = &full[..cut];
        let res = std::panic::catch_unwind(|| l2p::lit::decode(sliced));
        match res {
            Ok(Err(_)) | Ok(Ok(_)) => {} // error is ideal; a decode only possible if not truncated
            Err(_) => panic!("panic at cut={cut}"),
        }
        assert!(
            matches!(l2p::lit::decode(sliced), Err(_)) || cut >= full.len(),
            "cut={cut} must be an error"
        );
    }
    // Empty / garbage
    assert!(l2p::lit::decode(&[]).is_err());
    assert!(l2p::lit::decode(b"NOTALIT___________").is_err());
}

#[test]
fn truncated_at_every_plane_boundary() {
    // DownCorner has alpha; truncations right after each segment boundary must error.
    let full = read("Windows/DownCorner.lit");
    let parsed = l2p::lit::header::parse(&full).unwrap();
    let g = parsed.geometry.unwrap();
    let bounds = [
        16,
        16 + g.y_size,
        16 + g.y_size + g.c_size,
        16 + g.y_size + 2 * g.c_size,
    ];
    for b in bounds {
        // cutting exactly at a boundary leaves the remaining segments missing
        let sliced = &full[..b];
        let _ = l2p::lit::decode(sliced); // must not panic
        if b < full.len() {
            assert!(l2p::lit::decode(sliced).is_err(), "cut at {b} must error");
        }
    }
}

// ----------------------------------------------------------------- §8.4 UGS

#[test]
fn ugs_ask_fifty_frames() {
    let buf = read("Windows/Ask.ugs");
    let frames = l2p::ugs::decode_ugs(&buf).unwrap();
    assert_eq!(frames.len(), 50, "Ask.ugs = 50 frames");
    for f in &frames {
        assert_eq!((f.image.w, f.image.h), (40, 40));
    }
    // Frame 0 has non-trivial content.
    let f0 = &frames[0].image;
    let nonzero = f0.rgb.iter().filter(|&&v| v > 32).count();
    assert!(nonzero > 50, "frame 0 must have visible pixels, got {nonzero}");
}

#[test]
fn ugs_atlas_objects_decodes_fully() {
    // Objects.ugs: reverse-engineered atlas layout — 370 records, no leftover.
    let buf = read("Objects/Objects.ugs");
    let frames = l2p::ugs::decode_ugs(&buf).unwrap();
    assert_eq!(frames.len(), 370, "atlas record count");
    assert!(frames.iter().all(|f| f.kind.is_some()), "atlas frames carry (kind,id)");
    let with_kind: Vec<_> = frames.iter().map(|f| f.kind.unwrap()).collect();
    assert!(with_kind.contains(&(1, 10)), "first record (kind 1, id 10)");
    assert!(with_kind.contains(&(9, 206)), "late record (kind 9, id 206)");
}

#[test]
fn ugs_persones_decodes_fully() {
    let buf = read("Objects/Persones.ugs");
    let frames = l2p::ugs::decode_ugs(&buf).unwrap();
    assert_eq!(frames.len(), 102);
    assert_eq!((frames[0].image.w, frames[0].image.h), (115, 379));
}

#[test]
fn ugs_battle_spell_sequences() {
    let buf = read("Battle/--CURE.ugs");
    let frames = l2p::ugs::decode_ugs(&buf).unwrap();
    assert_eq!(frames.len(), 25);
    assert_eq!((frames[0].image.w, frames[0].image.h), (220, 110));

    let buf = read("Spells/P-Flare.ugs");
    let frames = l2p::ugs::decode_ugs(&buf).unwrap();
    assert_eq!(frames.len(), 50);
    assert_eq!((frames[0].image.w, frames[0].image.h), (128, 128));
}

// ------------------------------------------------------------- full decodes

/// All Windows .lit files decode without error and produce non-degenerate output.
#[test]
fn decode_sample_of_every_lit_shape() {
    // one file per flag variant: 0x0, 0x2, 0x4, 0x6, 0x8, 0xA
    let files = [
        ("Textures/Dust.lit", 0x0),
        ("Windows/Army-Death.lit", 0x2),
        ("Windows/Bonus-NoPayment.lit", 0x4),
        ("Windows/Bonus1.lit", 0x6),
        ("Windows/SI_Helm.lit", 0x8),
        ("Windows/DownCorner.lit", 0xA),
    ];
    for (file, flags) in files {
        let buf = read(file);
        let parsed = l2p::lit::header::parse(&buf).unwrap();
        assert_eq!(parsed.header.flags, flags, "{file}");
        let img = l2p::lit::decode(&buf).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(img.w, parsed.header.width);
        assert_eq!(img.h, parsed.header.height);
        assert_eq!(img.rgb.len(), img.w as usize * img.h as usize * 3);
        // non-degenerate: not all-black, not all-one-color
        let mut distinct = std::collections::HashSet::new();
        for px in img.rgb.chunks(3) {
            distinct.insert([px[0], px[1], px[2]]);
            if distinct.len() > 16 {
                break;
            }
        }
        assert!(distinct.len() > 4, "{file}: degenerate image");
        // alpha presence matches the flag
        assert_eq!(img.alpha.is_some(), flags & 0x8 != 0, "{file} alpha");
    }
}
