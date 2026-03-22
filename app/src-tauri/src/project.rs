use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

const ENGINE_CORE_DTS: &str = include_str!("../../../modules/quickjs_scripting/engine-core.d.ts");

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<String>,
}

/// Standard project directory structure
const PROJECT_DIRS: &[&str] = &["scripts", "assets", "build"];

#[tauri::command]
pub async fn create_project(parent_dir: String, name: String) -> Result<ProjectInfo, String> {
    let project_path = PathBuf::from(&parent_dir).join(&name);

    // Create project directory and subdirectories
    for dir in PROJECT_DIRS {
        fs::create_dir_all(project_path.join(dir))
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Create a default main.ts
    let main_ts = r#"// Main entry point
const node = new Engine.Node3D();
node.name = "Root";
Engine.SceneTree.get_root().add_child(node);
console.log("Hello from LLM3dEngine!");
"#;
    fs::write(project_path.join("scripts/main.ts"), main_ts)
        .await
        .map_err(|e| format!("Failed to write main.ts: {}", e))?;

    // Create tsconfig.json for the game project
    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ES2020",
    "strict": true,
    "outDir": "build",
    "rootDir": ".",
    "declaration": false,
    "skipLibCheck": true
  },
  "include": ["scripts/**/*.ts", "engine-core.d.ts", "globals.d.ts"]
}
"#;
    fs::write(project_path.join("tsconfig.json"), tsconfig)
        .await
        .map_err(|e| format!("Failed to write tsconfig.json: {}", e))?;

    // Copy engine-core.d.ts into project
    fs::write(project_path.join("engine-core.d.ts"), ENGINE_CORE_DTS)
        .await
        .map_err(|e| format!("Failed to write engine-core.d.ts: {}", e))?;

    // Create engine globals shim so Engine can be used as a value
    let globals_dts = r#"// Engine runtime globals - allows using Engine.* as values
declare const Engine: {
    [key: string]: any;
};
"#;
    fs::write(project_path.join("globals.d.ts"), globals_dts)
        .await
        .map_err(|e| format!("Failed to write globals.d.ts: {}", e))?;

    Ok(ProjectInfo {
        name,
        path: project_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn open_project(path: String) -> Result<ProjectInfo, String> {
    let project_path = PathBuf::from(&path);
    if !project_path.exists() {
        return Err("Project path does not exist".into());
    }

    let name = project_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    Ok(ProjectInfo {
        name,
        path: project_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn write_project_file(
    project_path: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    let full_path = PathBuf::from(&project_path).join(&relative_path);

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(&full_path, &content)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn read_project_file(
    project_path: String,
    relative_path: String,
) -> Result<String, String> {
    let full_path = PathBuf::from(&project_path).join(&relative_path);
    fs::read_to_string(&full_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn compile_project(project_path: String) -> Result<CompileResult, String> {
    let project_dir = Path::new(&project_path);

    // Use tsc with --project flag pointing to the game project's tsconfig
    // Resolve tsc from the app's node_modules (sibling to src-tauri)
    let app_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let local_tsc = app_dir.join("node_modules/.bin/tsc");

    let tsc_cmd = if local_tsc.exists() {
        local_tsc.to_string_lossy().to_string()
    } else {
        "tsc".to_string()
    };

    let output = Command::new(&tsc_cmd)
        .arg("--noEmit")
        .arg("--pretty")
        .arg("--project")
        .arg(project_dir.join("tsconfig.json"))
        .output()
        .await
        .map_err(|e| format!("Failed to run tsc: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    let errors: Vec<String> = combined
        .lines()
        .filter(|l| l.contains("error TS"))
        .map(|l| l.to_string())
        .collect();

    Ok(CompileResult {
        success: output.status.success(),
        output: combined,
        errors,
    })
}
