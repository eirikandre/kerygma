//! `kerygma` fra terminalen: samme funksjoner som appen, uten vindu.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kerygma_core::bible::{Bibel, BibelConfig};
use kerygma_core::presentation::{presentation_content, presentation_paths};
use kerygma_core::settings::Settings;
use kerygma_core::verse_query::lookup_verse;
use kerygma_core::word::{import_docx, note_name, ConvertOptions};

const HELP: &str = "\
kerygma - manus og presentasjoner fra terminalen

BRUK
    kerygma lag-presentasjon <manus.md> [FLAGG]
                        skriver presentasjonen til stdout
    kerygma vers <referanse eller s=søk>
                        slår opp som «Sett inn bibelvers»
    kerygma importer <fil.docx eller mappe>... [--ut <mappe>] [--uten-søk]
                        gjør Word-manus om til markdown ved siden av .docx-fila

FLAGG
    --skriv             skriv presentasjonen ved siden av manuset i stedet
    --maks <N>          maks tegn per slide (standard 350)
    --oversettelse <id> oversettelsen som legges ved siden av (standard kjv)
    --uten-oversettelse ikke legg til oversettelse
    --bibler <mappe>    hvor *.bin ligger (standard $KERYGMA_BIBLER eller ./bibler)
    --ut <mappe>        hvor importerte manus legges (standard: ved siden av .docx-fila)
    --uten-søk          ikke søk etter bibelvers i avsnitt uten referanse
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("kerygma: {message}");
            ExitCode::FAILURE
        }
    }
}

struct Options {
    positional: Vec<String>,
    write: bool,
    out: Option<PathBuf>,
    search: bool,
    settings: Settings,
    bibles: PathBuf,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        positional: Vec::new(),
        write: false,
        out: None,
        search: true,
        settings: Settings::default(),
        bibles: std::env::var_os("KERYGMA_BIBLER").map_or_else(|| PathBuf::from("bibler"), PathBuf::from),
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| args.next().cloned().ok_or_else(|| format!("{flag} mangler verdi"));
        match arg.as_str() {
            "--skriv" => options.write = true,
            "--uten-søk" => options.search = false,
            "--ut" => options.out = Some(PathBuf::from(value("--ut")?)),
            "--uten-oversettelse" => options.settings.add_translation = false,
            "--maks" => {
                options.settings.max_chars =
                    value("--maks")?.parse().map_err(|_| "--maks må være et positivt tall".to_string())?
            }
            "--oversettelse" => options.settings.translation = value("--oversettelse")?,
            "--bibler" => options.bibles = PathBuf::from(value("--bibler")?),
            flag if flag.starts_with("--") => return Err(format!("ukjent flagg {flag}\n\n{HELP}")),
            _ => options.positional.push(arg.clone()),
        }
    }
    options.settings = options.settings.sanitized();
    Ok(options)
}

fn load_bibel(dir: &Path, settings: &Settings) -> Result<Bibel, String> {
    let mut bibel = Bibel::load_dir(dir)?;
    bibel.configure(&BibelConfig {
        default_translation: settings.default_translation.clone(),
        aliases: settings.aliases.clone(),
    })?;
    Ok(bibel)
}

fn run(args: &[String]) -> Result<(), String> {
    let Some((command, rest)) = args.split_first() else {
        print!("{HELP}");
        return Ok(());
    };
    let options = parse_options(rest)?;
    match command.as_str() {
        "lag-presentasjon" => make_presentation(&options),
        "vers" => verse(&options),
        "importer" => import(&options),
        "hjelp" | "--hjelp" | "-h" | "--help" => {
            print!("{HELP}");
            Ok(())
        }
        other => Err(format!("ukjent kommando «{other}»\n\n{HELP}")),
    }
}

fn make_presentation(options: &Options) -> Result<(), String> {
    let [path] = options.positional.as_slice() else {
        return Err("bruk: kerygma lag-presentasjon <manus.md>".to_string());
    };
    let path = path.replace('\\', "/");
    let paths = presentation_paths(&path, &options.settings.suffix);
    let manuscript = std::fs::read_to_string(&paths.manuscript)
        .map_err(|e| format!("kunne ikke lese «{}»: {e}", paths.manuscript))?;

    let bibel = if options.settings.add_translation {
        match load_bibel(&options.bibles, &options.settings) {
            Ok(bibel) => Some(bibel),
            Err(message) => {
                eprintln!("kerygma: {message}");
                None
            }
        }
    } else {
        None
    };
    let (content, warnings) = presentation_content(&manuscript, &options.settings, bibel.as_ref());
    for warning in warnings {
        eprintln!("kerygma: {warning}");
    }
    if options.write {
        std::fs::write(&paths.presentation, content)
            .map_err(|e| format!("kunne ikke skrive «{}»: {e}", paths.presentation))?;
        eprintln!("Presentasjon laget: {}", paths.presentation);
    } else {
        print!("{content}");
    }
    Ok(())
}

fn verse(options: &Options) -> Result<(), String> {
    let input = options.positional.join(" ");
    let bibel = load_bibel(&options.bibles, &options.settings)?;
    for suggestion in lookup_verse(&bibel, &input) {
        match (suggestion.text, suggestion.message) {
            (Some(text), _) => println!("{text}"),
            (None, Some(message)) => println!("[{message}]"),
            (None, None) => {}
        }
    }
    Ok(())
}

fn import(options: &Options) -> Result<(), String> {
    if options.positional.is_empty() {
        return Err("bruk: kerygma importer <fil.docx eller mappe>...".to_string());
    }
    let mut files = Vec::new();
    for path in options.positional.iter().map(PathBuf::from) {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(&path)
                .map_err(|e| format!("kunne ikke lese «{}»: {e}", path.display()))?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    name.to_lowercase().ends_with(".docx") && !name.starts_with("~$")
                })
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path);
        }
    }
    let bibel = load_bibel(&options.bibles, &options.settings)?;
    let convert_options = ConvertOptions { search: options.search };
    for file in files {
        let data = std::fs::read(&file).map_err(|e| format!("kunne ikke lese «{}»: {e}", file.display()))?;
        let result = import_docx(&data, &bibel, &convert_options).map_err(|e| format!("{}: {e}", file.display()))?;
        let name = note_name(&file.file_name().unwrap_or_default().to_string_lossy()) + ".md";
        let dir = options.out.clone().unwrap_or_else(|| file.parent().map(Path::to_path_buf).unwrap_or_default());
        std::fs::create_dir_all(&dir).map_err(|e| format!("kunne ikke lage «{}»: {e}", dir.display()))?;
        let out = dir.join(name);
        std::fs::write(&out, &result.markdown).map_err(|e| format!("kunne ikke skrive «{}»: {e}", out.display()))?;
        let uncertain = result.quotes.iter().filter(|q| q.warning.is_some()).count();
        println!("{}: {} bibelsitater, {uncertain} med advarsel", out.display(), result.quotes.len());
        if !result.unmatched.is_empty() {
            println!("  referanser uten sitat: {}", result.unmatched.join(", "));
        }
    }
    Ok(())
}
