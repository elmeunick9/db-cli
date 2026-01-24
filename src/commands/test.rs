use std::path::Path;
use std::collections::HashMap;
use crate::utils::db::{self, DbPool};
use crate::utils::fs;
use crate::config::Config;
use tracing::{info, error};

/// Test result for a single test function
#[derive(Debug, Clone)]
struct TestResult {
    test_name: String,
    file_path: String,
    passed: bool,
    output: String,
    line_number: Option<usize>,
}

/// Extract the line number from test output and test file contents
fn extract_line_number(output: &str, test_file_contents: &HashMap<String, Vec<String>>, file_path: &str) -> Option<usize> {
    // Parse the error message to find the anchor
    if let Some(anchor_match) = output.split(']').next() {
        if let Some(anchor) = anchor_match.strip_prefix('[') {
            // Find the line with this anchor in the test file
            if let Some(lines) = test_file_contents.get(file_path) {
                for (line_num, line) in lines.iter().enumerate() {
                    if line.contains(anchor) {
                        return Some(line_num + 1); // 1-indexed
                    }
                }
            }
        }
    }
    None
}

/// Create a test database at the specified version
async fn create_test_db(
    config: &Config,
    test_db_name: &str,
    version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the test database using the standard init flow
    crate::commands::init::execute(&config, Some(version.to_string())).await?;
    
    // Execute all test files to create the test functions
    let sql_base = config.sql_base();
    let schemas = fs::list_schemas(&sql_base, version)?;
    let pool = db::get_db_pool(&config).await?;
    
    for schema in schemas {
        let files = fs::read_schema_files(&sql_base, version, &schema)?;
        for file in &files {
            if file.path.to_string_lossy().ends_with(".test.sql") {
                db::run_raw_with_schema(&pool, &config, &file.content, Some(&schema)).await?;
            }
        }
    }
    
    info!("Created test database: {}", test_db_name);
    Ok(())
}

/// Run a single test function and return its output (empty string = pass, non-empty = fail)
async fn run_test_function(
    pool: &DbPool,
    test_function_name: &str,
    schema: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    
    // Call the test function and get the result
    let sql = format!("SELECT \"{}\".\"{}\"();", schema, test_function_name);
    
    match pool {
        DbPool::Postgres(p) => {
            let result: String = sqlx::query_scalar(&sql)
                .fetch_one(p)
                .await?;
            Ok(result)
        }
        DbPool::DryRun => Ok("".to_string()),
    }
}

/// Drop the test database
async fn drop_test_db(
    config: &Config,
    test_db_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to maintenance DB to drop the test DB
    let mut maintenance = config.clone();
    maintenance.database.name = config.database.maintenance_db_name.clone();
    let pool = db::get_db_pool(&maintenance).await?;
    
    let drop_sql = format!("DROP DATABASE IF EXISTS \"{}\"", test_db_name);
    db::run(&pool, &maintenance, &drop_sql).await?;
    
    info!("Dropped test database: {}", test_db_name);
    Ok(())
}

/// Run tests in the database
pub async fn execute(config: &Config, version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let sql_base = config.sql_base();
    let mut version = version.unwrap_or_else(|| "next".to_string());

    // Resolve version aliases like "latest" to a concrete version folder
    if version == "latest" {
        match fs::get_latest_version(sql_base) {
            Ok(v) => {
                version = v;
            }
            Err(e) => {
                return Err(format!("failed to resolve 'latest' version: {}", e).into());
            }
        }
    }

    info!("Running tests for version: {}", version);

    let version_dir = Path::new(sql_base).join(&version);
    if !version_dir.exists() {
        return Err(format!("Version directory not found: {}", version_dir.display()).into());
    }

    // Find all test files
    let test_files = fs::find_test_files(sql_base, &version)?;
    if test_files.is_empty() {
        info!("No test files found for version {}", version);
        return Ok(());
    }

    info!("Found {} test files", test_files.len());
    
    // Create pool for test database operations
    let test_db_name = format!("testdb_{}", config.database.name);
    let mut config = config.clone();
    config.database.name = test_db_name.to_string();
    config.log_sql = false;
    
    // Create test database
    if let Err(e) = create_test_db(&config, &test_db_name, &version).await {
        eprintln!("Failed to create test database: {}", e);
        return Err(e);
    }
    let pool = db::get_db_pool(&config).await?;
    
    // Read and store test file content for later reference
    let mut test_file_contents: HashMap<String, Vec<String>> = HashMap::new();
    for (test_file, _) in &test_files {
        if let Ok(content) = std::fs::read_to_string(&test_file) {
            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            test_file_contents.insert(test_file.clone(), lines);
        }
    }

    // Run tests
    let mut results = vec![];
    let mut passed_count = 0;
    let mut failed_count = 0;

    for (test_file, schema) in test_files {
        // Extract test name from filename (e.g., "comment_vote.test.sql" -> "comment_vote_test")
        if let Some(file_stem) = Path::new(&test_file).file_stem() {
            if let Some(name_without_test) = file_stem.to_string_lossy().strip_suffix(".test") {
                let test_function_name = format!("{}_test", name_without_test);

                // Run the test function
                match run_test_function(&pool,  &test_function_name, &schema).await {
                    Ok(output) => {
                        if output.is_empty() || output.trim() == "OK" {
                            passed_count += 1;
                            results.push(TestResult {
                                test_name: test_function_name.clone(),
                                file_path: test_file.clone(),
                                passed: true,
                                output,
                                line_number: None,
                            });
                        } else {
                            failed_count += 1;
                            let line_number = extract_line_number(&output, &test_file_contents, &test_file);
                            results.push(TestResult {
                                test_name: test_function_name.clone(),
                                file_path: test_file.clone(),
                                passed: false,
                                output,
                                line_number,
                            });
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        let error_output = format!("Error running test: {}", e);
                        results.push(TestResult {
                            test_name: test_function_name.clone(),
                            file_path: test_file.clone(),
                            passed: false,
                            output: error_output,
                            line_number: None,
                        });
                    }
                }
            }
        }
    }

    // Print results
    println!("\n{}", "=".repeat(80));
    println!("Test Results");
    println!("{}", "=".repeat(80));

    for result in &results {
        if result.passed {
            println!("\nPASS: {}", result.test_name);
        } else {
            println!("\nFAIL: {}", result.test_name);
            
            // Print file:line in VS Code format
            if let Some(line_num) = result.line_number {
                println!("{}:{}", result.file_path, line_num);
                
                // Print context (3 lines before and after)
                if let Some(lines) = test_file_contents.get(&result.file_path) {
                    let start = if line_num > 4 { line_num - 4 } else { 1 };
                    let end = std::cmp::min(line_num + 3, lines.len());
                    
                    println!("\nContext:");
                    for i in (start - 1)..end {
                        let prefix = if i == line_num - 1 { ">>> " } else { "    " };
                        println!("{}{}:{}", prefix, i + 1, lines[i]);
                    }
                }
            } else {
                println!("File: {}", result.file_path);
            }

            // Print the error message
            println!("\nMessage: {}", result.output);
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("Summary: {} passed, {} failed", passed_count, failed_count);
    println!("{}", "=".repeat(80));

    // Clean up test database
    drop(pool);
    if let Err(e) = drop_test_db(&config, &test_db_name).await {
        error!("Failed to drop test database: {}", e);
    }

    // Exit with error code if any tests failed
    if failed_count > 0 {
        return Err(format!("{} test(s) failed", failed_count).into());
    }

    Ok(())
}
