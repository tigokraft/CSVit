use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Inferred data type for a column
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InferredType {
    Integer,
    Float,
    Boolean,
    Date,
    Text,
    Empty,
    Mixed,
}

impl InferredType {
    pub fn name(&self) -> &'static str {
        match self {
            InferredType::Integer => "Integer",
            InferredType::Float => "Float",
            InferredType::Boolean => "Boolean",
            InferredType::Date => "Date",
            InferredType::Text => "Text",
            InferredType::Empty => "Empty",
            InferredType::Mixed => "Mixed",
        }
    }
}

/// Profile/statistics for a single column
#[derive(Clone, Debug, Default)]
pub struct ColumnProfile {
    pub column_index: usize,
    pub header: String,
    pub data_type: Option<InferredType>,
    pub total_count: usize,
    pub null_count: usize,
    pub unique_count: usize,
    // Numeric stats
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub sum: Option<f64>,
    pub mean: Option<f64>,
    pub std_dev: Option<f64>,
    // Categorical stats (top 5 values)
    pub top_values: Vec<(String, usize)>,
}

impl ColumnProfile {
    pub fn null_percentage(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.null_count as f64 / self.total_count as f64) * 100.0
        }
    }
}

/// Analyzer that profiles CSV columns
pub struct ColumnAnalyzer;

impl ColumnAnalyzer {
    /// Analyze a column from a grid
    pub fn analyze_column(
        header: &str,
        col_index: usize,
        values: &[String],
    ) -> ColumnProfile {
        let mut profile = ColumnProfile {
            column_index: col_index,
            header: header.to_string(),
            total_count: values.len(),
            ..Default::default()
        };

        if values.is_empty() {
            profile.data_type = Some(InferredType::Empty);
            return profile;
        }

        // Count nulls and collect non-null values
        let mut non_null_values: Vec<&str> = Vec::new();
        let mut value_counts: HashMap<String, usize> = HashMap::new();

        for val in values {
            let trimmed = val.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed.eq_ignore_ascii_case("na") || trimmed.eq_ignore_ascii_case("n/a") {
                profile.null_count += 1;
            } else {
                non_null_values.push(trimmed);
                *value_counts.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }

        profile.unique_count = value_counts.len();

        // Top values
        let mut top: Vec<(String, usize)> = value_counts.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        profile.top_values = top.into_iter().take(5).collect();

        // Infer type and compute stats
        let (inferred_type, numeric_values) = Self::infer_type(&non_null_values);
        profile.data_type = Some(inferred_type.clone());

        // Compute numeric stats if applicable
        if !numeric_values.is_empty() {
            let sum: f64 = numeric_values.iter().sum();
            let count = numeric_values.len() as f64;
            let mean = sum / count;

            let variance: f64 = numeric_values.iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>() / count;
            let std_dev = variance.sqrt();

            profile.min = numeric_values.iter().cloned().reduce(f64::min);
            profile.max = numeric_values.iter().cloned().reduce(f64::max);
            profile.sum = Some(sum);
            profile.mean = Some(mean);
            profile.std_dev = Some(std_dev);
        }

        profile
    }

    /// Infer the type of a column based on its values
    fn infer_type(values: &[&str]) -> (InferredType, Vec<f64>) {
        if values.is_empty() {
            return (InferredType::Empty, vec![]);
        }

        let mut int_count = 0;
        let mut float_count = 0;
        let mut bool_count = 0;
        let mut date_count = 0;
        let mut text_count = 0;
        let mut numeric_values = Vec::new();

        for val in values {
            // Try integer
            if val.parse::<i64>().is_ok() {
                int_count += 1;
                if let Ok(n) = val.parse::<f64>() {
                    numeric_values.push(n);
                }
                continue;
            }

            // Try float
            if val.parse::<f64>().is_ok() {
                float_count += 1;
                if let Ok(n) = val.parse::<f64>() {
                    numeric_values.push(n);
                }
                continue;
            }

            // Try boolean
            let lower = val.to_lowercase();
            if lower == "true" || lower == "false" || lower == "yes" || lower == "no" || lower == "1" || lower == "0" {
                bool_count += 1;
                continue;
            }

            // Try date patterns (simple check)
            if val.contains('-') || val.contains('/') {
                let parts: Vec<&str> = val.split(|c| c == '-' || c == '/').collect();
                if parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok()) {
                    date_count += 1;
                    continue;
                }
            }

            // Otherwise text
            text_count += 1;
        }

        let total = values.len();
        let int_ratio = int_count as f64 / total as f64;
        let float_ratio = float_count as f64 / total as f64;
        let bool_ratio = bool_count as f64 / total as f64;
        let date_ratio = date_count as f64 / total as f64;

        // Determine type (80% threshold)
        if int_ratio > 0.8 {
            (InferredType::Integer, numeric_values)
        } else if (int_ratio + float_ratio) > 0.8 {
            (InferredType::Float, numeric_values)
        } else if bool_ratio > 0.8 {
            (InferredType::Boolean, vec![])
        } else if date_ratio > 0.8 {
            (InferredType::Date, vec![])
        } else if text_count > 0 || total == text_count {
            (InferredType::Text, vec![])
        } else {
            (InferredType::Mixed, numeric_values)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_column() {
        let values: Vec<String> = vec!["1", "2", "3", "4", "5"]
            .into_iter()
            .map(String::from)
            .collect();
        let profile = ColumnAnalyzer::analyze_column("Numbers", 0, &values);
        
        assert_eq!(profile.data_type, Some(InferredType::Integer));
        assert_eq!(profile.min, Some(1.0));
        assert_eq!(profile.max, Some(5.0));
        assert_eq!(profile.mean, Some(3.0));
    }

    #[test]
    fn test_null_count() {
        let values: Vec<String> = vec!["1", "", "3", "null", "5"]
            .into_iter()
            .map(String::from)
            .collect();
        let profile = ColumnAnalyzer::analyze_column("WithNulls", 0, &values);
        
        assert_eq!(profile.null_count, 2);
        assert_eq!(profile.total_count, 5);
    }
}
