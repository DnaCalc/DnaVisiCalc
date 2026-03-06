#[derive(Debug, Clone, PartialEq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub values: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, values: Vec<f64>) -> Self {
        Self { rows, cols, values }
    }

    pub fn scalar(value: f64) -> Self {
        Self {
            rows: 1,
            cols: 1,
            values: vec![value],
        }
    }

    pub fn value_at(&self, row: usize, col: usize) -> f64 {
        self.values[row * self.cols + col]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayNodeValue {
    Scalar(f64),
    Matrix(Matrix),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayGraphError {
    ShapeMismatch,
}

pub fn elementwise_add(
    lhs: &ArrayNodeValue,
    rhs: &ArrayNodeValue,
) -> Result<ArrayNodeValue, ArrayGraphError> {
    elementwise_binary(lhs, rhs, |a, b| a + b)
}

pub fn elementwise_binary(
    lhs: &ArrayNodeValue,
    rhs: &ArrayNodeValue,
    op: impl Fn(f64, f64) -> f64,
) -> Result<ArrayNodeValue, ArrayGraphError> {
    let left = to_matrix(lhs);
    let right = to_matrix(rhs);
    let (rows, cols) = broadcast_shape(left.rows, left.cols, right.rows, right.cols)?;

    let mut out = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        for col in 0..cols {
            let left_row = if left.rows == 1 { 0 } else { row };
            let left_col = if left.cols == 1 { 0 } else { col };
            let right_row = if right.rows == 1 { 0 } else { row };
            let right_col = if right.cols == 1 { 0 } else { col };
            out.push(op(
                left.value_at(left_row, left_col),
                right.value_at(right_row, right_col),
            ));
        }
    }

    if rows == 1 && cols == 1 {
        Ok(ArrayNodeValue::Scalar(out[0]))
    } else {
        Ok(ArrayNodeValue::Matrix(Matrix::new(rows, cols, out)))
    }
}

fn to_matrix(value: &ArrayNodeValue) -> Matrix {
    match value {
        ArrayNodeValue::Scalar(v) => Matrix::scalar(*v),
        ArrayNodeValue::Matrix(matrix) => matrix.clone(),
    }
}

fn broadcast_shape(
    left_rows: usize,
    left_cols: usize,
    right_rows: usize,
    right_cols: usize,
) -> Result<(usize, usize), ArrayGraphError> {
    if left_rows == right_rows && left_cols == right_cols {
        return Ok((left_rows, left_cols));
    }
    if left_rows == 1 && left_cols == 1 {
        return Ok((right_rows, right_cols));
    }
    if right_rows == 1 && right_cols == 1 {
        return Ok((left_rows, left_cols));
    }
    Err(ArrayGraphError::ShapeMismatch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcasts_scalar_over_matrix() {
        let lhs = ArrayNodeValue::Matrix(Matrix::new(2, 2, vec![1.0, 2.0, 3.0, 4.0]));
        let rhs = ArrayNodeValue::Scalar(10.0);
        let out = elementwise_add(&lhs, &rhs).expect("add");
        assert_eq!(
            out,
            ArrayNodeValue::Matrix(Matrix::new(2, 2, vec![11.0, 12.0, 13.0, 14.0]))
        );
    }

    #[test]
    fn rejects_incompatible_shapes() {
        let lhs = ArrayNodeValue::Matrix(Matrix::new(2, 1, vec![1.0, 2.0]));
        let rhs = ArrayNodeValue::Matrix(Matrix::new(1, 2, vec![3.0, 4.0]));
        let out = elementwise_add(&lhs, &rhs);
        assert_eq!(out, Err(ArrayGraphError::ShapeMismatch));
    }
}
