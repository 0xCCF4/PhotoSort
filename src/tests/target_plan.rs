//! Tests for the planning of the target names.

use crate::action::{ActionMode, ActualAction};
use crate::analysis::exif2date::ExifDateType;
use crate::analysis::filename2date::NaiveFileNameParser;
use crate::analysis::name_formatters::{
    BracketingFormattingPriority, FormatDuplicate, FormatExtension, FormatName,
};
use crate::target_plan::{AnalyzedFile, Plan, PlanKind, Planner};
use crate::testutils::TempDir;
use crate::{AnalysisType, Analyzer, AnalyzerSettings};
use std::fs;
use std::path::{Path, PathBuf};

fn write(path: &Path, content: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, content).expect("write file");
}

fn analyzer(target: &Path, action: ActionMode, format: &str) -> Analyzer {
    let settings = AnalyzerSettings {
        analysis_type: AnalysisType::OnlyName,
        exif_date_type: ExifDateType::Creation,
        time_zone: crate::analysis::time_zone::TimeZone::default(),
        output_time_zone: crate::analysis::time_zone::TimeZone::default(),
        source_dirs: Vec::new(),
        target_dir: target.to_path_buf(),
        recursive_source: true,
        fast: false,
        dont_follow_symlinked_dirs: false,
        dont_follow_symlinked_files: false,
        file_format: format.to_string(),
        nodate_file_format: format.to_string(),
        unknown_file_format: None,
        bracketed_file_format: None,
        date_format: "%Y%m%d-%H%M%S".to_string(),
        bracketing_formatting: BracketingFormattingPriority::default(),
        extensions: vec!["jpg".to_string()],
        #[cfg(feature = "video")]
        video_extensions: vec!["mp4".to_string()],
        action_type: action,
        mkdir: false,
        excluded_files: Vec::new(),
        included_files: Vec::new(),
    };

    let mut analyzer = Analyzer::new(settings).expect("create analyzer");
    analyzer.add_transformer(NaiveFileNameParser::default());
    analyzer.add_formatter(FormatName::default());
    analyzer.add_formatter(FormatDuplicate::default());
    analyzer.add_formatter(FormatExtension::default());
    analyzer
}

fn analyzed(analyzer: &Analyzer, source: &Path) -> AnalyzedFile {
    let info = analyzer
        .analyze_context(source)
        .expect("analyze file")
        .expect("file is not skipped");
    AnalyzedFile {
        source: source.to_path_buf(),
        info,
        bracket: None,
    }
}

fn plan_of(analyzer: &Analyzer, sources: &[PathBuf]) -> Plan {
    let files = sources.iter().map(|s| analyzed(analyzer, s)).collect();
    Planner::batch(analyzer).plan(files)
}

#[test]
fn identical_sources_are_deduplicated() {
    let dir = TempDir::new("identical");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    write(&first, b"same content");
    write(&second, b"same content");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[first.clone(), second.clone()]);

    assert!(
        plan.errors.is_empty(),
        "unexpected errors: {:?}",
        plan.errors
    );
    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[0].kind, PlanKind::Action);
    assert_eq!(plan.entries[1].kind, PlanKind::DropDuplicate);
    assert_eq!(plan.entries[0].target, plan.entries[1].target);
    assert_eq!(plan.entries[0].target.file_name().unwrap(), "a.jpg");
}

#[test]
fn different_sources_get_dup_numbers() {
    let dir = TempDir::new("differ");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    write(&first, b"content one");
    write(&second, b"content two");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[first, second]);

    assert!(
        plan.errors.is_empty(),
        "unexpected errors: {:?}",
        plan.errors
    );
    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[0].kind, PlanKind::Action);
    assert_eq!(plan.entries[1].kind, PlanKind::Action);
    assert_eq!(plan.entries[0].target.file_name().unwrap(), "a.jpg");
    assert_eq!(plan.entries[1].target.file_name().unwrap(), "a-1.jpg");
}

#[test]
fn missing_dup_format_reports_an_error() {
    let dir = TempDir::new("nodup");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    write(&first, b"content one");
    write(&second, b"content two");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}.{ext}",
    );
    let plan = plan_of(&analyzer, &[first, second]);

    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.errors.len(), 1);
    assert!(plan.errors[0].1.contains("already exists"));
}

#[test]
fn identical_existing_target_is_a_duplicate() {
    let dir = TempDir::new("existing_identical");
    let out = dir.path().join("out");
    write(&out.join("a.jpg"), b"same content");

    let source = dir.path().join("s1/20240101_120000_a.jpg");
    write(&source, b"same content");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[source]);

    assert!(
        plan.errors.is_empty(),
        "unexpected errors: {:?}",
        plan.errors
    );
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].kind, PlanKind::DropDuplicate);
    assert_eq!(plan.entries[0].target.file_name().unwrap(), "a.jpg");
}

#[test]
fn different_existing_target_gets_a_dup_number() {
    let dir = TempDir::new("existing_different");
    let out = dir.path().join("out");
    write(&out.join("a.jpg"), b"other content");

    let source = dir.path().join("s1/20240101_120000_a.jpg");
    write(&source, b"same content");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[source]);

    assert!(
        plan.errors.is_empty(),
        "unexpected errors: {:?}",
        plan.errors
    );
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].kind, PlanKind::Action);
    assert_eq!(plan.entries[0].target.file_name().unwrap(), "a-1.jpg");
}

#[test]
fn hardlinked_sources_are_deduplicated() {
    let dir = TempDir::new("hardlink");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    write(&first, b"link content");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::hard_link(&first, &second).expect("create hardlink");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[first, second]);

    assert_eq!(plan.entries.len(), 2);
    assert_eq!(plan.entries[1].kind, PlanKind::DropDuplicate);
}

#[test]
fn planned_order_decides_the_numbers() {
    let dir = TempDir::new("order");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    write(&first, b"content one");
    write(&second, b"content two");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );

    // The first source in the plan wins the number, independent of its path.
    let plan = plan_of(&analyzer, &[second, first]);
    assert_eq!(plan.entries[0].target.file_name().unwrap(), "a.jpg");
    assert_eq!(plan.entries[1].target.file_name().unwrap(), "a-1.jpg");
}

#[test]
fn executing_the_plan_writes_one_file() {
    let dir = TempDir::new("execute");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    write(&first, b"same content");
    write(&second, b"same content");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[first, second]);

    for entry in &plan.entries {
        analyzer.execute_entry(entry).expect("execute entry");
    }

    assert!(out.join("a.jpg").exists());
    assert!(!out.join("a-1.jpg").exists());
    assert_eq!(fs::read(out.join("a.jpg")).unwrap(), b"same content");
}

#[test]
fn move_mode_drops_the_duplicate_source() {
    let dir = TempDir::new("move");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    write(&first, b"same content");
    write(&second, b"same content");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Move),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[first.clone(), second.clone()]);

    for entry in &plan.entries {
        analyzer.execute_entry(entry).expect("execute entry");
    }

    assert!(!first.exists());
    assert!(!second.exists());
    assert_eq!(fs::read(out.join("a.jpg")).unwrap(), b"same content");
    assert!(!out.join("a-1.jpg").exists());
}

#[test]
fn copy_mode_leaves_the_duplicate_source() {
    let dir = TempDir::new("copy_keep");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let first = dir.path().join("s1/20240101_120000_a.jpg");
    let second = dir.path().join("s2/20240101_120000_a.jpg");
    write(&first, b"same content");
    write(&second, b"same content");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}{-:dup}.{ext}",
    );
    let plan = plan_of(&analyzer, &[first.clone(), second.clone()]);

    for entry in &plan.entries {
        analyzer.execute_entry(entry).expect("execute entry");
    }

    assert!(first.exists());
    assert!(second.exists());
}

/// A target that appears between planning and execution must not be
/// overwritten, and the error must name the file.
#[test]
fn a_target_that_appears_late_is_not_overwritten() {
    let dir = TempDir::new("late_target");
    let out = dir.path().join("out");
    fs::create_dir_all(&out).unwrap();

    let source = dir.path().join("s1/20240101_120000_a.jpg");
    write(&source, b"same content");

    let analyzer = analyzer(
        &out,
        ActionMode::Execute(ActualAction::Copy),
        "{name}.{ext}",
    );
    let plan = plan_of(&analyzer, &[source]);
    assert_eq!(plan.entries.len(), 1);

    write(&out.join("a.jpg"), b"other content");

    let err = analyzer
        .execute_entry(&plan.entries[0])
        .expect_err("must not overwrite a target that appeared late");
    let message = err.to_string();

    assert!(message.contains("already exists"), "message: {message}");
    assert!(message.contains("a.jpg"), "message: {message}");
    assert!(
        message.contains("Nothing was overwritten"),
        "message: {message}"
    );
    assert_eq!(fs::read(out.join("a.jpg")).unwrap(), b"other content");
}
