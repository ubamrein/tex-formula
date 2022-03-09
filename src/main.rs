// Copyright (c) 2022 Patrick Amrein <amrein@ubique.ch>
// 
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT


use std::{process::{Command, Stdio}, io::{Write, Cursor}, borrow::Cow};

use arboard::{Clipboard, ImageData};
use image::{DynamicImage};
use image::io::Reader as ImageReader;

use clap::Parser;
use tectonic::tt_error;

#[derive(Parser, Debug)]
#[clap(author = "Patrick Amrein", version = "1.0", about = "Copy latex formula to the clipboard", long_about = None)]
struct Args {
    #[clap(short, long)]
    font_size: Option<u32>,
    formula: String,
}
fn main() {
    let input = Args::parse();
    let latex = format!(
        r#"
\documentclass[border=4pt,preview]{{standalone}}
\usepackage{{mathtools}}
\usepackage{{amsthm}}
\fontsize{{ {} }}{{12}}\selectfont
\begin{{document}}
    \begin{{align*}}
    {}
    \end{{align*}}
\end{{document}}
"#,
        input.font_size.unwrap_or(12),
        input.formula
    );
    
    let pdf_data: Vec<u8> = match tectonic::latex_to_pdf(latex) {
        Ok(data) => data,
        Err(e) => {
            panic!("{:?}",e )
        },
    };
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
        .spawn().unwrap();
       

    let mut stdin = command.stdin.take().unwrap();
  
    stdin.write_all(&pdf_data).unwrap();
    drop(stdin);
    let stdout = command.wait_with_output().unwrap();
    let png = stdout.stdout;
    let dynamic_image : DynamicImage = ImageReader::new(Cursor::new(&png)).with_guessed_format().unwrap().decode().unwrap();
    let mut clip = Clipboard::new().unwrap();
    let bytes = dynamic_image.to_rgba8().to_vec();
    let image_data = ImageData {
        width: dynamic_image.width() as usize,
        height: dynamic_image.height() as usize,
        bytes: Cow::Owned(bytes)
    };
    clip.set_image(image_data).unwrap();
}
