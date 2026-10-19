use std::path::{Path, PathBuf};

use dbx_core::table_import::{
    parse_xlsx_file_with_options, preview_table_import_file_with_request, TableImportParseOptions,
    TableImportPreviewRequest,
};
use dbx_formats::xlsx_export::{build_xlsx_workbook_multi, XlsxWorksheetData};
use serde_json::{json, Value};

fn worksheet(name: &str, columns: &[&str], rows: Vec<Vec<Value>>) -> XlsxWorksheetData {
    XlsxWorksheetData {
        sheet_name: Some(name.to_string()),
        columns: columns.iter().map(|column| column.to_string()).collect(),
        column_types: vec![],
        column_comments: vec![],
        rows,
        numeric_column_right_align: false,
        auto_filter: None,
        header_comment_rows: false,
    }
}

fn workbook_file() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sheets.xlsx");
    let bytes = build_xlsx_workbook_multi(&[
        worksheet("EmptyFirst", &[], vec![]),
        worksheet("Data", &["id"], vec![vec![json!(7)]]),
        worksheet("HeaderOnly", &["id"], vec![]),
        worksheet("EmptyLast", &[], vec![]),
    ])
    .unwrap();
    std::fs::write(&path, bytes).unwrap();
    (directory, path)
}

fn request(path: &Path, options: TableImportParseOptions) -> TableImportPreviewRequest {
    TableImportPreviewRequest {
        file_path: path.to_string_lossy().into_owned(),
        connection_id: None,
        database: None,
        source_ref: None,
        source_format: None,
        parse_options: options,
        preview_limit: Some(10),
    }
}

#[tokio::test]
async fn empty_excel_preview_preserves_sheet_choices_without_allowing_empty_imports() {
    let (_directory, path) = workbook_file();
    let expected_sheets = ["EmptyFirst", "Data", "HeaderOnly", "EmptyLast"];
    for sheet_name in [None, Some("EmptyFirst"), Some("HeaderOnly"), Some("EmptyLast")] {
        let options = TableImportParseOptions {
            sheet_name: sheet_name.map(str::to_string),
            ..TableImportParseOptions::default()
        };
        let preview = preview_table_import_file_with_request(request(&path, options.clone())).await.unwrap();
        assert_eq!(preview.sheets, expected_sheets);
        assert!(preview.rows.is_empty());
        assert_eq!(preview.total_rows, 0);
        assert!(preview.total_rows_exact);
        assert!(parse_xlsx_file_with_options(&path.to_string_lossy(), &options, 10).is_err());
    }

    let options =
        TableImportParseOptions { sheet_name: Some("Data".to_string()), ..TableImportParseOptions::default() };
    let preview = preview_table_import_file_with_request(request(&path, options.clone())).await.unwrap();
    assert_eq!(preview.sheets, expected_sheets);
    assert_eq!(preview.columns, ["id"]);
    assert_eq!(preview.rows, [vec![json!(7)]]);
    assert_eq!(parse_xlsx_file_with_options(&path.to_string_lossy(), &options, 10).unwrap().rows, preview.rows);
}

#[tokio::test]
async fn empty_excel_row_range_keeps_the_workbook_selectable() {
    let (_directory, path) = workbook_file();
    let options = TableImportParseOptions {
        sheet_name: Some("Data".to_string()),
        data_start_row: Some(20),
        ..TableImportParseOptions::default()
    };
    let preview = preview_table_import_file_with_request(request(&path, options.clone())).await.unwrap();
    assert_eq!(preview.sheets, ["EmptyFirst", "Data", "HeaderOnly", "EmptyLast"]);
    assert!(preview.rows.is_empty());
    assert!(preview.total_rows_exact);
    assert!(parse_xlsx_file_with_options(&path.to_string_lossy(), &options, 10).is_err());
}
