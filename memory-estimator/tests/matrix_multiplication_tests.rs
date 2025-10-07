
/// Test case creator for large matrix multiplication
/// Creates matrices similar to the Python function:
/// - matrix_a: 1000 x 20 matrix (1000 rows, 20 columns) 
/// - matrix_b: 20 x 150 matrix (20 rows, 150 columns)
fn create_large_matrices() -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    // Matrix A: 1000 x 20
    let mut matrix_a = Vec::new();
    for i in 0..1000 {
        let mut row = Vec::new();
        for j in 0..20 {
            row.push((i + j) as f64);
        }
        matrix_a.push(row);
    }
    
    // Matrix B: 20 x 150
    let mut matrix_b = Vec::new();
    for i in 0..20 {
        let mut row = Vec::new();
        for j in 0..150 {
            row.push((i * j) as f64);
        }
        matrix_b.push(row);
    }
    
    println!("Created large matrices: A({}x{}), B({}x{})", 
             matrix_a.len(), matrix_a[0].len(), 
             matrix_b.len(), matrix_b[0].len());
    
    (matrix_a, matrix_b)
}

