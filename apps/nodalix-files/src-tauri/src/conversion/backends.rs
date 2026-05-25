use super::config::ConversionConfig;
use super::registry::normalize_ext;
use super::tools::{detect_tools, resolved_program, ToolAvailability};
use crate::platform;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[derive(Debug)]
pub struct CommandRun {
    pub program: String,
    pub args: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub struct FileConversionService {
    config: ConversionConfig,
    tools: ToolAvailability,
}

impl FileConversionService {
    pub fn new(config: ConversionConfig) -> Self {
        let tools = detect_tools(&config);
        Self { config, tools }
    }

    pub fn tools(&self) -> &ToolAvailability {
        &self.tools
    }

    pub fn config(&self) -> &ConversionConfig {
        &self.config
    }

    pub fn convert(&self, source: &Path, dest: &Path, from_ext: &str, to_ext: &str) -> Result<(), String> {
        if dest.exists() {
            return Err(format!("Ya existe: {}", dest.display()));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let from = normalize_ext(from_ext);
        let to = normalize_ext(to_ext);
        platform::debug_log(&format!(
            "convert {} -> {} ({} -> {})",
            source.display(),
            dest.display(),
            from,
            to
        ));

        let result = match (from.as_str(), to.as_str()) {
            ("png", "jpg") => convert_image(source, dest, image::ImageFormat::Jpeg),
            ("jpg", "png") => convert_image(source, dest, image::ImageFormat::Png),
            ("webp", "png") => convert_image(source, dest, image::ImageFormat::Png),
            ("webp", "jpg") => convert_image(source, dest, image::ImageFormat::Jpeg),
            ("png", "webp") | ("jpg", "webp") => {
                convert_image(source, dest, image::ImageFormat::WebP)
            }
            ("bmp", "png") => convert_image(source, dest, image::ImageFormat::Png),
            ("tiff", "png") => convert_image(source, dest, image::ImageFormat::Png),
            ("txt", "md") | ("md", "txt") => copy_text(source, dest),
            ("csv", "txt") | ("json", "txt") | ("toml", "txt") | ("yaml", "txt") => {
                copy_text_with_encoding_check(source, dest)
            }
            ("docx", "pdf")
            | ("odt", "pdf")
            | ("pptx", "pdf")
            | ("xlsx", "pdf") => {
                self.convert_office_to_pdf(source, dest)
            }
            ("svg", "png") => self.convert_svg_to_png(source, dest),
            ("pdf", "png") => self.convert_pdf_to_png(source, dest),
            ("heic", "jpg") | ("heic", "png") => self.convert_heic(source, dest, &to),
            _ => Err(format!("Conversión no implementada: .{from} -> .{to}")),
        };

        if let Err(error) = &result {
            platform::debug_log(&format!("convert failed: {error}"));
        } else if dest.exists() {
            platform::debug_log(&format!("convert ok: {}", dest.display()));
        } else {
            return Err(format!(
                "La conversión no generó el archivo esperado: {}",
                dest.display()
            ));
        }
        result
    }
}

fn convert_image(source: &Path, dest: &Path, format: image::ImageFormat) -> Result<(), String> {
    let img = image::open(source).map_err(|e| format!("No se pudo leer la imagen: {e}"))?;
    if format == image::ImageFormat::Jpeg {
        let rgb = img.to_rgb8();
        let mut file = fs::File::create(dest).map_err(|e| e.to_string())?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 90);
        encoder
            .encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| format!("No se pudo escribir JPEG: {e}"))?;
        return Ok(());
    }
    img.save_with_format(dest, format)
        .map_err(|e| format!("No se pudo guardar la imagen: {e}"))
}

fn copy_text(source: &Path, dest: &Path) -> Result<(), String> {
    fs::copy(source, dest).map_err(|e| e.to_string())?;
    Ok(())
}

fn copy_text_with_encoding_check(source: &Path, dest: &Path) -> Result<(), String> {
    let bytes = fs::read(source).map_err(|e| e.to_string())?;
    if std::str::from_utf8(&bytes).is_err() {
        platform::debug_log(&format!(
            "text conversion warning: non-UTF-8 source {}",
            source.display()
        ));
    }
    fs::write(dest, bytes).map_err(|e| e.to_string())?;
    Ok(())
}

impl FileConversionService {
    fn convert_office_to_pdf(&self, source: &Path, dest: &Path) -> Result<(), String> {
        if !self.tools.libreoffice {
            return Err("LibreOffice no está instalado.".into());
        }
        let out_dir = dest
            .parent()
            .ok_or_else(|| "No se pudo resolver la carpeta de destino".to_string())?;
        let program = resolved_program(&self.config, "libreoffice", "libreoffice");
        let output = run_command_logged(
            &program,
            &[
                "--headless",
                "--convert-to",
                "pdf",
                "--outdir",
                &path_to_string(out_dir),
                &path_to_string(source),
            ],
        )?;
        if !output.success {
            return Err(format_command_failure(&output));
        }
        let expected = out_dir.join(source.file_stem().unwrap_or_default()).with_extension("pdf");
        if expected == dest {
            if expected.exists() {
                return Ok(());
            }
        } else if expected.exists() {
            fs::rename(&expected, dest).map_err(|e| e.to_string())?;
            return Ok(());
        }
        if dest.exists() {
            return Ok(());
        }
        Err(format!(
            "LibreOffice no generó el PDF esperado. Log: stdout={} stderr={}",
            output.stdout, output.stderr
        ))
    }

    fn convert_svg_to_png(&self, source: &Path, dest: &Path) -> Result<(), String> {
        if !self.tools.rsvg_convert {
            return Err("rsvg-convert no está instalado.".into());
        }
        let program = resolved_program(&self.config, "rsvg-convert", "rsvg-convert");
        let output = run_command_logged(
            &program,
            &["-o", &path_to_string(dest), &path_to_string(source)],
        )?;
        if output.success && dest.exists() {
            Ok(())
        } else {
            Err(format_command_failure(&output))
        }
    }

    fn convert_pdf_to_png(&self, source: &Path, dest: &Path) -> Result<(), String> {
        if !self.tools.pdftoppm {
            return Err("pdftoppm no está instalado.".into());
        }
        let program = resolved_program(&self.config, "pdftoppm", "pdftoppm");
        let output_base = dest.with_extension("");
        let output = run_command_logged(
            &program,
            &[
                "-f",
                "1",
                "-singlefile",
                "-png",
                "-scale-to",
                "1024",
                &path_to_string(source),
                &path_to_string(&output_base),
            ],
        )?;
        if output.success && dest.exists() {
            Ok(())
        } else {
            Err(format_command_failure(&output))
        }
    }

    fn convert_heic(&self, source: &Path, dest: &Path, to_ext: &str) -> Result<(), String> {
        if self.tools.imagemagick {
            let program = resolved_program(&self.config, "magick", "magick");
            let output = run_command_logged(
                &program,
                &[&path_to_string(source), &path_to_string(dest)],
            )?;
            if output.success && dest.exists() {
                return Ok(());
            }
            if !output.success {
                platform::debug_log(&format!("magick heic failed: {}", output.stderr));
            }
        }
        if self.tools.heif_convert {
            let program = resolved_program(&self.config, "heif-convert", "heif-convert");
            let output = run_command_logged(
                &program,
                &[&path_to_string(source), &path_to_string(dest)],
            )?;
            if output.success && dest.exists() {
                return Ok(());
            }
            return Err(format_command_failure(&output));
        }
        let _ = to_ext;
        Err("No hay backend HEIC disponible (magick o heif-convert).".into())
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn run_command_logged(program: &str, args: &[&str]) -> Result<CommandRun, String> {
    platform::debug_log(&format!("run: {program} {}", args.join(" ")));
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("No se pudo ejecutar {program}: {e}"))?;
    log_output(program, &output);
    Ok(CommandRun {
        program: program.to_string(),
        args: args.iter().map(|s| (*s).to_string()).collect(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    })
}

fn log_output(program: &str, output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.trim().is_empty() {
        platform::debug_log(&format!("{program} stdout: {stdout}"));
    }
    if !stderr.trim().is_empty() {
        platform::debug_log(&format!("{program} stderr: {stderr}"));
    }
}

fn format_command_failure(run: &CommandRun) -> String {
    format!(
        "Falló {} {} (code n/a). stdout={} stderr={}",
        run.program,
        run.args.join(" "),
        run.stdout.trim(),
        run.stderr.trim()
    )
}

#[cfg(test)]
mod image_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn png_to_jpg_produces_jpeg() {
        let mut buffer = Cursor::new(Vec::new());
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("test.png");
        let dst = dir.path().join("test.jpg");
        fs::write(&src, buffer.into_inner()).unwrap();
        convert_image(&src, &dst, image::ImageFormat::Jpeg).unwrap();
        assert!(dst.exists());
        let format = image::guess_format(&fs::read(&dst).unwrap()).unwrap();
        assert_eq!(format, image::ImageFormat::Jpeg);
    }
}
