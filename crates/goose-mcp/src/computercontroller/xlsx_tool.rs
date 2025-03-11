use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use umya_spreadsheet::{Spreadsheet, Worksheet};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorksheetInfo {
    name: String,
    index: usize,
    column_count: usize,
    row_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CellValue {
    value: String,
    formula: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RangeData {
    start_row: u32,
    end_row: u32,
    start_col: u32,
    end_col: u32,
    // First dimension is rows, second dimension is columns: values[row_index][column_index]
    values: Vec<Vec<CellValue>>,
}

pub struct XlsxTool {
    workbook: Spreadsheet,
}

impl XlsxTool {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let workbook =
            umya_spreadsheet::reader::xlsx::read(path).context("Failed to read Excel file")?;
        Ok(Self { workbook })
    }

    pub fn list_worksheets(&self) -> Result<Vec<WorksheetInfo>> {
        let mut worksheets = Vec::new();
        for (index, worksheet) in self.workbook.get_sheet_collection().iter().enumerate() {
            let (column_count, row_count) = self.get_worksheet_dimensions(worksheet)?;
            worksheets.push(WorksheetInfo {
                name: worksheet.get_name().to_string(),
                index,
                column_count,
                row_count,
            });
        }
        Ok(worksheets)
    }

    pub fn get_worksheet_by_name(&self, name: &str) -> Result<&Worksheet> {
        self.workbook
            .get_sheet_by_name(name)
            .context("Worksheet not found")
    }

    pub fn get_worksheet_by_index(&self, index: usize) -> Result<&Worksheet> {
        self.workbook
            .get_sheet_collection()
            .get(index)
            .context("Worksheet index out of bounds")
    }

    fn get_worksheet_dimensions(&self, worksheet: &Worksheet) -> Result<(usize, usize)> {
        // Returns (column_count, row_count) for the worksheet
        let mut max_col = 0;
        let mut max_row = 0;

        // Iterate through all rows
        for row_num in 1..=worksheet.get_highest_row() {
            for col_num in 1..=worksheet.get_highest_column() {
                if let Some(cell) = worksheet.get_cell((row_num, col_num)) {
                    let coord = cell.get_coordinate();
                    max_col = max_col.max(*coord.get_col_num() as usize);
                    max_row = max_row.max(*coord.get_row_num() as usize);
                }
            }
        }

        Ok((max_col, max_row))
    }

    pub fn get_column_names(&self, worksheet: &Worksheet) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for col_num in 1..=worksheet.get_highest_column() {
            if let Some(cell) = worksheet.get_cell((1, col_num)) {
                names.push(cell.get_value().into_owned());
            } else {
                names.push(String::new());
            }
        }
        Ok(names)
    }

    pub fn get_range(&self, worksheet: &Worksheet, range: &str) -> Result<RangeData> {
        let (start_col, start_row, end_col, end_row) = parse_range(range)?;
        let mut values = Vec::new();

        // Iterate through rows first, then columns
        for row_idx in start_row..=end_row {
            let mut row_values = Vec::new();
            for col_idx in start_col..=end_col {
                let cell_value = if let Some(cell) = worksheet.get_cell((row_idx, col_idx)) {
                    CellValue {
                        value: cell.get_value().into_owned(),
                        formula: if cell.get_formula().is_empty() {
                            None
                        } else {
                            Some(cell.get_formula().to_string())
                        },
                    }
                } else {
                    CellValue {
                        value: String::new(),
                        formula: None,
                    }
                };
                row_values.push(cell_value);
            }
            values.push(row_values);
        }

        Ok(RangeData {
            start_row,
            end_row,
            start_col,
            end_col,
            values,
        })
    }

    pub fn update_cell(
        &mut self,
        worksheet_name: &str,
        row: u32,
        col: u32,
        value: &str,
    ) -> Result<()> {
        let worksheet = self
            .workbook
            .get_sheet_by_name_mut(worksheet_name)
            .context("Worksheet not found")?;

        worksheet
            .get_cell_mut((row, col))
            .set_value(value.to_string());
        Ok(())
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        umya_spreadsheet::writer::xlsx::write(&self.workbook, path)
            .context("Failed to save Excel file")?;
        Ok(())
    }

    pub fn find_in_worksheet(
        &self,
        worksheet: &Worksheet,
        search_text: &str,
        case_sensitive: bool,
    ) -> Result<Vec<(u32, u32)>> {
        // Returns a vector of (row, column) coordinates where matches are found
        let mut matches = Vec::new();
        let search_text = if !case_sensitive {
            search_text.to_lowercase()
        } else {
            search_text.to_string()
        };

        for row_num in 1..=worksheet.get_highest_row() {
            for col_num in 1..=worksheet.get_highest_column() {
                if let Some(cell) = worksheet.get_cell((row_num, col_num)) {
                    let cell_value = if !case_sensitive {
                        cell.get_value().to_lowercase()
                    } else {
                        cell.get_value().to_string()
                    };

                    if cell_value.contains(&search_text) {
                        let coord = cell.get_coordinate();
                        matches.push((*coord.get_row_num(), *coord.get_col_num()));
                    }
                }
            }
        }

        Ok(matches)
    }

    pub fn get_cell_value(&self, worksheet: &Worksheet, row: u32, col: u32) -> Result<CellValue> {
        let cell = worksheet.get_cell((row, col)).context("Cell not found")?;

        Ok(CellValue {
            value: cell.get_value().into_owned(),
            formula: if cell.get_formula().is_empty() {
                None
            } else {
                Some(cell.get_formula().to_string())
            },
        })
    }
}

fn parse_range(range: &str) -> Result<(u32, u32, u32, u32)> {
    // Handle ranges like "A1:B10"
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid range format. Expected format: 'A1:B10'");
    }

    let start = parse_cell_reference(parts[0])?;
    let end = parse_cell_reference(parts[1])?;

    Ok((start.0, start.1, end.0, end.1))
}

fn parse_cell_reference(reference: &str) -> Result<(u32, u32)> {
    // Parse Excel cell reference (e.g., "A1") and return (column, row)
    let mut col_str = String::new();
    let mut row_str = String::new();
    let mut parsing_row = false;

    for c in reference.chars() {
        if c.is_alphabetic() {
            if parsing_row {
                anyhow::bail!("Invalid cell reference format");
            }
            col_str.push(c.to_ascii_uppercase());
        } else if c.is_numeric() {
            parsing_row = true;
            row_str.push(c);
        } else {
            anyhow::bail!("Invalid character in cell reference");
        }
    }

    let col = column_letter_to_number(&col_str)?;
    let row = row_str.parse::<u32>().context("Invalid row number")?;

    Ok((col, row))
}

fn column_letter_to_number(column: &str) -> Result<u32> {
    let mut result = 0u32;
    for c in column.chars() {
        if !c.is_ascii_alphabetic() {
            anyhow::bail!("Invalid column letter");
        }
        result = result * 26 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_file() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("computercontroller")
            .join("tests")
            .join("data")
            .join("FinancialSample.xlsx")
    }

    #[test]
    fn test_open_xlsx() -> Result<()> {
        let xlsx = XlsxTool::new(get_test_file())?;
        let worksheets = xlsx.list_worksheets()?;
        assert!(!worksheets.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_column_names() -> Result<()> {
        let xlsx = XlsxTool::new(get_test_file())?;
        let worksheet = xlsx.get_worksheet_by_index(0)?;
        let columns = xlsx.get_column_names(worksheet)?;
        assert!(!columns.is_empty());
        println!("Columns: {:?}", columns);
        Ok(())
    }

    #[test]
    fn test_get_range() -> Result<()> {
        let xlsx = XlsxTool::new(get_test_file())?;
        let worksheet = xlsx.get_worksheet_by_index(0)?;
        let range = xlsx.get_range(worksheet, "A1:C5")?;
        assert_eq!(range.values.len(), 5);
        println!("Range data: {:?}", range);
        Ok(())
    }

    #[test]
    fn test_find_in_worksheet() -> Result<()> {
        let xlsx = XlsxTool::new(get_test_file())?;
        let worksheet = xlsx.get_worksheet_by_index(0)?;
        let matches = xlsx.find_in_worksheet(worksheet, "Government", false)?;
        assert!(!matches.is_empty());
        println!("Found matches at: {:?}", matches);
        Ok(())
    }

    #[test]
    fn test_get_cell_value() -> Result<()> {
        let xlsx = XlsxTool::new(get_test_file())?;
        let worksheet = xlsx.get_worksheet_by_index(0)?;

        // Test header cell (known value from FinancialSample.xlsx)
        let header_cell = xlsx.get_cell_value(worksheet, 1, 1)?;
        assert_eq!(header_cell.value, "Segment");
        assert!(header_cell.formula.is_none());

        // Test data cell (known value from FinancialSample.xlsx)
        let data_cell = xlsx.get_cell_value(worksheet, 2, 2)?;
        assert_eq!(data_cell.value, "Canada");
        assert!(data_cell.formula.is_none());

        println!(
            "Header cell: {:#?}\nData cell: {:#?}",
            header_cell, data_cell
        );
        Ok(())
    }
}
