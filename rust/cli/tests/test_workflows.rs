use std::fs;
use std::path::PathBuf;
use axm_cli::run;

fn p(name: &str, ext: &str) -> PathBuf {
    let mut pb = PathBuf::from("target");
    pb.push(format!("{}_{}.{}", name, std::process::id(), ext));
    let _ = fs::create_dir_all(pb.parent().unwrap());
    pb
}

#[test]
fn e2e_sim_stats_replay_export_verify() {
    // 1) simulate
    let out_jsonl = p("wf_sim", "jsonl");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(["axm","sim","--hands","3","--seed","4","--output", out_jsonl.to_string_lossy().as_ref()], &mut out, &mut err);
    assert_eq!(code, 0, "sim should exit 0, stderr={}", String::from_utf8_lossy(&err));
    let contents = fs::read_to_string(&out_jsonl).unwrap();
    assert_eq!(contents.lines().filter(|l| !l.trim().is_empty()).count(), 3);

    // 2) stats
    out.clear(); err.clear();
    let code = run(["axm","stats","--input", out_jsonl.to_string_lossy().as_ref()], &mut out, &mut err);
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"hands\": 3"));

    // 3) replay
    out.clear(); err.clear();
    let code = run(["axm","replay","--input", out_jsonl.to_string_lossy().as_ref()], &mut out, &mut err);
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Replayed: 3 hands"));

    // 4) export json
    let out_json = p("wf_exp", "json");
    out.clear(); err.clear();
    let code = run(["axm","export","--input", out_jsonl.to_string_lossy().as_ref(), "--format","json","--output", out_json.to_string_lossy().as_ref()], &mut out, &mut err);
    assert_eq!(code, 0);
    let arr: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out_json).unwrap()).unwrap();
    assert_eq!(arr.as_array().unwrap().len(), 3);

    // 5) verify OK for completed boards
    out.clear(); err.clear();
    let code = run(["axm","verify","--input", out_jsonl.to_string_lossy().as_ref()], &mut out, &mut err);
    assert_eq!(code, 0, "verify should be OK: stderr={} out={}", String::from_utf8_lossy(&err), String::from_utf8_lossy(&out));
}

