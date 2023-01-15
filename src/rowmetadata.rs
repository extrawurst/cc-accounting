use crate::project::CsvRow;
use std::path::Path;

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct RowMetaData {
    pub hidden: bool,
    pub receipt: Option<String>,
}

impl RowMetaData {
    pub fn rename_pdf(&mut self, idx: usize, row: &CsvRow) {
        let target_name = self.target_file_name(idx, row);
        if let Some(receipt) = self.receipt.as_mut() {
            let target_name = target_name.expect("cannot happen since receipt is not none");

            tracing::debug!("rename pdf: '{}' -> '{}'", receipt, target_name);

            std::fs::rename(receipt.clone(), target_name.clone()).expect("TODO");
            *receipt = target_name;
        }
    }

    pub fn is_name_correct(&self, idx: usize, row: &CsvRow) -> bool {
        let target_name = self.target_file_name(idx, row);
        if let Some(receipt) = self.receipt.as_ref() {
            target_name.map(|f| f == *receipt).unwrap_or(false)
        } else {
            // no receipt means the name is correct
            true
        }
    }

    fn target_file_name(&self, idx: usize, row: &CsvRow) -> Option<String> {
        if let Some(receipt) = self.receipt.as_ref() {
            let receipt_path = Path::new(receipt);
            let date = row.cells[0].clone();
            let amount = row.cells[3].clone();
            let entry_name = row.cells[2].clone().replace('/', "_");
            let target_name = format!(
                "{}/{:0>3}-{}{}EUR-{}.pdf",
                receipt_path.parent().unwrap().to_str().unwrap(),
                idx,
                date,
                amount,
                entry_name,
            );

            Some(target_name)
        } else {
            None
        }
    }

    pub fn get_receipt_filename(&self) -> Option<&str> {
        self.receipt
            .as_ref()
            .and_then(|r| Path::new(r).file_name().and_then(|f| f.to_str()))
    }
}
