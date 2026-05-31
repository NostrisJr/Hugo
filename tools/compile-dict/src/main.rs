//! Compilation de Dicollecte (Hunspell `.dic`/`.aff`) en FST de formes.
//!
//! Lit le dictionnaire orthographique français de Dicollecte (format Hunspell,
//! MPL 2.0), développe les **suffixes** (pluriels, féminins, conjugaisons…) et
//! écrit l'ensemble trié et dédupliqué des formes fléchies dans un FST
//! (`fst::Set`) consommable par `hugo-core`.
//!
//! Les **préfixes** du dictionnaire français sont exclusivement des élisions
//! (`l'`, `d'`, `qu'`…) et des variantes de casse ; comme le tokenizer de Hugo
//! sépare déjà les élisions et recherche le mot de base, ils ne sont pas
//! développés.
//!
//! Usage : `compile-dict <fr.dic> <fr.aff> <sortie.fst>`
//!
//! Hypothèses sur le format (vérifiées sur le dictionnaire « classique » v7) :
//! `SET UTF-8`, `FLAG long` (drapeaux de 2 caractères), `FULLSTRIP`,
//! `NEEDAFFIX ()`, `FORBIDDENWORD {}`.

use std::collections::{BTreeSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::process::ExitCode;

/// Un atome d'une condition d'affixe (porte sur un caractère).
#[derive(Debug, Clone)]
enum Atom {
    /// `.` — n'importe quel caractère.
    Any,
    /// `[abc]` ou `[^abc]` — appartenance (ou non) à un ensemble.
    Set { negate: bool, chars: Vec<char> },
    /// Un caractère littéral.
    Lit(char),
}

/// Une règle de suffixation.
#[derive(Debug, Clone)]
struct SuffixEntry {
    /// Lettres à retirer de la fin (vide si « 0 »).
    strip: Vec<char>,
    /// Lettres à ajouter (vide si « 0 »).
    add: String,
    /// Drapeaux de continuation (suffixation au second tour).
    cont_flags: Vec<String>,
    /// Condition de fin de mot.
    condition: Vec<Atom>,
}

/// Une classe de suffixes (toutes les règles partageant un drapeau).
#[derive(Debug, Default, Clone)]
struct SuffixClass {
    entries: Vec<SuffixEntry>,
}

/// Découpe une chaîne de drapeaux en drapeaux de 2 caractères (`FLAG long`).
fn parse_flags_long(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    chars.chunks(2).map(|c| c.iter().collect()).collect()
}

/// Analyse une condition Hunspell (`.`, `[^sxz]`, `aeu`…).
fn parse_condition(s: &str) -> Vec<Atom> {
    if s == "." {
        return vec![Atom::Any];
    }
    let mut atoms = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '.' => atoms.push(Atom::Any),
            '[' => {
                let mut negate = false;
                if chars.peek() == Some(&'^') {
                    negate = true;
                    chars.next();
                }
                let mut set = Vec::new();
                for d in chars.by_ref() {
                    if d == ']' {
                        break;
                    }
                    set.push(d);
                }
                atoms.push(Atom::Set { negate, chars: set });
            }
            other => atoms.push(Atom::Lit(other)),
        }
    }
    atoms
}

/// Vrai si la fin de `word` (suite de caractères) satisfait la condition.
fn condition_matches_end(word: &[char], condition: &[Atom]) -> bool {
    if condition.len() > word.len() {
        return false;
    }
    let tail = &word[word.len() - condition.len()..];
    tail.iter().zip(condition).all(|(c, atom)| match atom {
        Atom::Any => true,
        Atom::Lit(l) => c == l,
        Atom::Set { negate, chars } => chars.contains(c) != *negate,
    })
}

/// Applique une règle de suffixe à un mot, si elle est applicable.
fn apply_suffix(word: &[char], entry: &SuffixEntry) -> Option<String> {
    // La condition porte sur le mot avant retrait.
    if !condition_matches_end(word, &entry.condition) {
        return None;
    }
    if !entry.strip.is_empty() {
        if entry.strip.len() > word.len() {
            return None;
        }
        if &word[word.len() - entry.strip.len()..] != entry.strip.as_slice() {
            return None;
        }
    }
    let keep = word.len() - entry.strip.len();
    let mut out: String = word[..keep].iter().collect();
    out.push_str(&entry.add);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Métadonnées extraites de l'en-tête `.aff`.
struct AffixData {
    suffixes: std::collections::HashMap<String, SuffixClass>,
    needaffix: Option<String>,
    forbidden: Option<String>,
}

/// Analyse le fichier `.aff` : ne retient que les classes SFX.
fn parse_aff(path: &Path) -> std::io::Result<AffixData> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut suffixes: std::collections::HashMap<String, SuffixClass> =
        std::collections::HashMap::new();
    let mut needaffix = None;
    let mut forbidden = None;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim_end();
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("NEEDAFFIX") => needaffix = fields.next().map(|s| s.to_string()),
            Some("FORBIDDENWORD") => forbidden = fields.next().map(|s| s.to_string()),
            Some("SFX") => {
                let flag = match fields.next() {
                    Some(f) => f.to_string(),
                    None => continue,
                };
                let third = fields.next().unwrap_or("");
                // En-tête de classe : `SFX flag Y|N count`.
                if third == "Y" || third == "N" {
                    suffixes.entry(flag).or_default();
                    continue;
                }
                // Règle : `SFX flag strip add[/cont] condition`.
                let strip = if third == "0" {
                    Vec::new()
                } else {
                    third.chars().collect()
                };
                let add_field = fields.next().unwrap_or("0");
                let (add_raw, cont) = match add_field.split_once('/') {
                    Some((a, c)) => (a, parse_flags_long(c)),
                    None => (add_field, Vec::new()),
                };
                let add = if add_raw == "0" {
                    String::new()
                } else {
                    add_raw.to_string()
                };
                let condition = parse_condition(fields.next().unwrap_or("."));
                suffixes.entry(flag).or_default().entries.push(SuffixEntry {
                    strip,
                    add,
                    cont_flags: cont,
                    condition,
                });
            }
            _ => {}
        }
    }

    Ok(AffixData {
        suffixes,
        needaffix,
        forbidden,
    })
}

/// Développe un radical et ses drapeaux en toutes ses formes suffixées.
///
/// Suffixation à deux tours (continuation), préfixes ignorés.
fn expand(stem: &str, flags: &[String], aff: &AffixData, out: &mut BTreeSet<String>) {
    let needs_affix = aff
        .needaffix
        .as_ref()
        .is_some_and(|n| flags.iter().any(|f| f == n));
    let forbidden = aff
        .forbidden
        .as_ref()
        .is_some_and(|n| flags.iter().any(|f| f == n));
    if forbidden {
        return;
    }
    // La forme nue est valide sauf si elle requiert un affixe.
    if !needs_affix {
        out.insert(stem.to_string());
    }

    // Parcours en largeur des suffixes, profondeur <= 2.
    let mut queue: VecDeque<(Vec<char>, Vec<String>, u8)> =
        VecDeque::from([(stem.chars().collect(), flags.to_vec(), 0u8)]);

    while let Some((word, word_flags, depth)) = queue.pop_front() {
        for flag in &word_flags {
            let Some(class) = aff.suffixes.get(flag) else {
                continue;
            };
            for entry in &class.entries {
                if let Some(form) = apply_suffix(&word, entry) {
                    out.insert(form.clone());
                    if depth + 1 < 2 && !entry.cont_flags.is_empty() {
                        queue.push_back((
                            form.chars().collect(),
                            entry.cont_flags.clone(),
                            depth + 1,
                        ));
                    }
                }
            }
        }
    }
}

/// Analyse une entrée `.dic` en `(mot, drapeaux)`.
fn parse_dic_line(line: &str) -> Option<(String, Vec<String>)> {
    // Champ utile = premier token (les champs morpho éventuels suivent).
    let token = line.split_whitespace().next()?;
    if token.is_empty() {
        return None;
    }
    // Séparation mot / drapeaux sur le premier « / » non échappé.
    let bytes = token.as_bytes();
    let mut slash = None;
    let mut prev_backslash = false;
    for (idx, &b) in bytes.iter().enumerate() {
        if b == b'/' && !prev_backslash {
            slash = Some(idx);
            break;
        }
        prev_backslash = b == b'\\';
    }
    match slash {
        Some(idx) => {
            let word = token[..idx].replace("\\/", "/");
            let flags = parse_flags_long(&token[idx + 1..]);
            Some((word, flags))
        }
        None => Some((token.replace("\\/", "/"), Vec::new())),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage : compile-dict <fr.dic> <fr.aff> <sortie.fst>");
        return Err("arguments invalides".into());
    }
    let dic_path = Path::new(&args[1]);
    let aff_path = Path::new(&args[2]);
    let out_path = Path::new(&args[3]);

    eprintln!("Lecture des affixes : {}", aff_path.display());
    let aff = parse_aff(aff_path)?;
    eprintln!("  {} classes de suffixes", aff.suffixes.len());

    eprintln!("Développement du dictionnaire : {}", dic_path.display());
    let dic = BufReader::new(File::open(dic_path)?);
    let mut forms: BTreeSet<String> = BTreeSet::new();
    let mut stems = 0usize;

    for (i, line) in dic.lines().enumerate() {
        let line = line?;
        // La première ligne est le nombre d'entrées.
        if i == 0 && line.trim().parse::<usize>().is_ok() {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some((word, flags)) = parse_dic_line(&line) {
            stems += 1;
            expand(&word, &flags, &aff, &mut forms);
        }
    }

    eprintln!("  {stems} radicaux → {} formes uniques", forms.len());

    eprintln!("Écriture du FST : {}", out_path.display());
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let writer = BufWriter::new(File::create(out_path)?);
    let mut builder = fst::SetBuilder::new(writer)?;
    for form in &forms {
        builder.insert(form)?;
    }
    builder.finish()?;

    let size = std::fs::metadata(out_path)?.len();
    eprintln!(
        "Terminé : {} ({:.1} Mo)",
        out_path.display(),
        size as f64 / 1_048_576.0
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Erreur : {e}");
            ExitCode::FAILURE
        }
    }
}
