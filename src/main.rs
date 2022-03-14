// Copyright (c) 2022 Patrick Amrein <amrein@ubique.ch>
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::{
    borrow::Cow,
    io::{Cursor, Write},
   
   
};
 #[cfg(feature = "ghostscript")]
 use std::process::{Command, Stdio};

use arboard::{Clipboard, ImageData};

use image::io::Reader as ImageReader;
use image::DynamicImage;
use clap::Parser;

#[cfg(feature = "with_poppler")]
use cairo::Format;

#[cfg(feature = "with_poppler")]
use poppler::{PopplerDocument, PopplerPage};

#[derive(Parser, Debug)]
#[clap(author = "Patrick Amrein", version = "1.0", about = "Copy latex formula to the clipboard", long_about = None)]
struct Args {
    #[clap(short, long)]
    font_size: Option<u32>,
    #[clap(short, long)]
    to_stdout: bool,
    #[clap(short, long)]
    editor: bool,
    #[clap(long, default_value = "code --wait")]
    editor_command: String,
    #[clap(default_value = "-")]
    formula: String,
    
}
fn main() {
    let input = Args::parse();
    let formula = if input.editor {
        let mut editor = dialoguer::Editor::new();
        editor.require_save(true);
        editor.extension("tex");
        editor.executable(&input.editor_command);
        if let Some(result) = editor.edit("Enter the formula").unwrap() {
            result.trim().to_string()
        } else {
            panic!("Abborted");
        }

    } else if input.formula == "-" {
        let mut input = String::new();
        loop {
            match std::io::stdin().read_line(&mut input) {
                Ok(len) => {
                    if len == 0 || input.ends_with("<<end\n") {
                        input = input.trim().replace("<<end", "").trim().to_string();
                        break input;
                    }
                }
                Err(error) => {
                    eprintln!("error: {}", error);
                    return;
                }
            };
        }
    } else {
        input.formula
    };
    println!("{}", formula);
    let latex = format!(
        r#"
\documentclass[border=4pt,preview]{{standalone}}
\usepackage{{mathtools}}
\usepackage{{amsthm}}
\DeclareMathOperator{{\lcm}}{{lcm}}
\fontsize{{ {} }}{{12}}\selectfont
\begin{{document}}
    \begin{{align*}}
    {}
    \end{{align*}}
\end{{document}}
"#,
        input.font_size.unwrap_or(12),
        formula
    );

    let mut pdf_data: Vec<u8> = match tectonic::latex_to_pdf(latex) {
        Ok(data) => data,
        Err(e) => {
            panic!("{:?}", e)
        }
    };
    let mut png = vec![];
    #[cfg(feature = "with_poppler")]
    {
        let doc: PopplerDocument = PopplerDocument::new_from_data(&mut pdf_data, "").unwrap();
        let page: PopplerPage = doc.get_page(0).unwrap();

        let (width, height) = page.get_size();
        let (width, height) = (4.0 * width, 4.0 * height);
        let surface =
            cairo::ImageSurface::create(Format::ARgb32, width as i32, height as i32).unwrap();
        surface.set_fallback_resolution(300.0, 300.0);
        surface.set_device_scale(4.0, 4.0);
        let ctxt = cairo::Context::new(&surface).unwrap();
        ctxt.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        ctxt.paint().unwrap();
        page.render_for_printing(&ctxt);

        surface.write_to_png(&mut png).unwrap();
    }

    #[cfg(feature = "ghostscript")]
    {
        let mut command = Command::new("gs")
            .arg("-q")
            .arg("-dSAFER")
            .arg("-r300")
            .arg("-sDEVICE=pngalpha")
            .arg("-sOutputFile=-")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdin = command.stdin.take().unwrap();

        stdin.write_all(&pdf_data).unwrap();
        drop(stdin);
        let stdout = command.wait_with_output().unwrap();
        png = stdout.stdout;
    }

    if input.to_stdout {
        std::io::stdout().write_all(&png).unwrap();
    } else {
        let dynamic_image: DynamicImage = ImageReader::new(Cursor::new(&png))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        let mut clip = Clipboard::new().unwrap();
        let bytes = dynamic_image.to_rgba8().to_vec();
        let image_data = ImageData {
            width: dynamic_image.width() as usize,
            height: dynamic_image.height() as usize,
            bytes: Cow::Owned(bytes),
        };
        clip.set_image(image_data).unwrap();
    }
}
