use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// 安装目录
fn install_dir() -> PathBuf {
    dirs_home().join(".refstore").join("bin")
}

fn dirs_home() -> PathBuf {
    // 优先用 HOME 环境变量，兼容 macOS / Linux
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// 检测当前 shell 的配置文件
fn detect_shell_configs() -> Vec<PathBuf> {
    let home = dirs_home();
    let mut configs = Vec::new();

    // 按优先级检测
    let candidates = [".zshrc", ".zprofile", ".zshenv", ".bashrc", ".bash_profile", ".profile"];

    for name in &candidates {
        let path = home.join(name);
        if path.exists() {
            configs.push(path);
        }
    }

    configs
}

/// 检查配置文件是否已经注册了 PATH
fn is_path_registered(config: &PathBuf, bin_dir: &PathBuf) -> Result<bool> {
    let content = fs::read_to_string(config)?;
    let export_line = format!("export PATH=\"$PATH:{}\"", bin_dir.display());
    Ok(content.contains(&export_line))
}

pub fn run() -> Result<()> {
    let current_exe = env::current_exe().context("Cannot determine current executable path")?;
    let bin_dir = install_dir();

    // 1. 创建安装目录
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("Failed to create directory: {}", bin_dir.display()))?;

    // 2. 复制二进制文件
    let dest = bin_dir.join("ref");
    fs::copy(&current_exe, &dest).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            current_exe.display(),
            dest.display()
        )
    })?;

    println!("Installed ref to {}", dest.display());

    // 3. 注册 PATH
    let configs = detect_shell_configs();

    if configs.is_empty() {
        println!("\nNo shell config file found (.zshrc, .bashrc, etc.).");
        println!(
            "Please add the following line to your shell config manually:"
        );
        println!(
            "  export PATH=\"$PATH:{}\"",
            bin_dir.display()
        );
        return Ok(());
    }

    let export_line = format!("export PATH=\"$PATH:{}\"", bin_dir.display());

    for config in &configs {
        if is_path_registered(config, &bin_dir)? {
            println!(
                "PATH already registered in {}",
                config.display()
            );
            continue;
        }

        // 追加到配置文件
        fs::OpenOptions::new()
            .append(true)
            .open(config)
            .and_then(|mut file| {
                use std::io::Write;
                writeln!(file)?;
                writeln!(file, "# refstore")?;
                writeln!(file, "{}", export_line)?;
                Ok(())
            })
            .with_context(|| format!("Failed to write to {}", config.display()))?;

        println!("Added PATH to {}", config.display());
    }

    println!("\nDone! Please restart your terminal or run:");
    println!("  source {}", configs[0].display());
    println!("\nThen you can use:");
    println!("  ref add --title \"My Paper\" --arxiv 2301.07041");

    Ok(())
}
