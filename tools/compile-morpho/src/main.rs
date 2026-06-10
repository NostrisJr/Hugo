//! Compilation d'un lexique morphologique en FST + blob d'analyses.
//!
//! Source : **Lexique383** (<http://www.lexique.org>, CC BY-SA 4.0), retenu car
//! le Lefff (choix initial de la feuille de route) n'est plus distribué à une
//! URL stable. Lexique fournit pour chaque forme sa catégorie (`cgram`), son
//! genre, son nombre et son lemme.
//!
//! Sorties (consommées par `hugo-core::morpho`) :
//! - `<base>.fst` : `fst::Map` associant chaque forme (en minuscules) à une
//!   valeur `u64` empaquetant `(offset << 8) | nombre_d_analyses` ;
//! - `<base>.bin` : blob d'enregistrements. Chaque analyse est sérialisée en
//!   `[cat u8][genre u8][nombre u8][personne u8][len u8][lemme UTF-8…]`.
//!
//! Usage : `compile-morpho <Lexique383.tsv> <base_de_sortie>`
//! (produit `<base_de_sortie>.fst` et `<base_de_sortie>.bin`).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

// Codes partagés avec `hugo-core::morpho`. Toute modification doit être
// répercutée des deux côtés.
mod codes {
    pub const CAT_UNKNOWN: u8 = 0;
    pub const CAT_NOUN: u8 = 1;
    pub const CAT_VERB: u8 = 2;
    pub const CAT_ADJ: u8 = 3;
    pub const CAT_DET: u8 = 4;
    pub const CAT_PRON: u8 = 5;
    pub const CAT_ADV: u8 = 6;
    pub const CAT_PREP: u8 = 7;
    pub const CAT_CONJ: u8 = 8;
    pub const CAT_INTERJ: u8 = 9;

    pub const GENDER_NONE: u8 = 0;
    pub const GENDER_MASC: u8 = 1;
    pub const GENDER_FEM: u8 = 2;

    pub const NUMBER_NONE: u8 = 0;
    pub const NUMBER_SING: u8 = 1;
    pub const NUMBER_PLUR: u8 = 2;
}

/// Une analyse morphologique compacte.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Analysis {
    cat: u8,
    gender: u8,
    number: u8,
    person: u8,
    /// Code mode+temps (0 = non fini / sans objet). Voir [`mood_tense_code`].
    mt: u8,
    lemma: String,
}

/// Encode un couple `mode:temps` de Lexique en un code compact partagé avec
/// `hugo-core::morpho`. Renvoie 0 pour un mode/temps non géré.
fn mood_tense_code(mode: &str, tense: &str) -> u8 {
    match (mode, tense) {
        ("ind", "pre") => 1,
        ("ind", "imp") => 2,
        ("ind", "fut") => 3,
        ("ind", "pas") => 4,
        ("cnd", "pre") => 5,
        ("sub", "pre") => 6,
        ("sub", "imp") => 7,
        ("imp", "pre") => 8,
        _ => 0,
    }
}

/// Décode un marqueur personne+nombre Lexique (« 1s », « 3p »…) en
/// `(personne, nombre)` codés. Renvoie `(0, 0)` si invalide.
fn parse_person_number(pn: &str) -> (u8, u8) {
    let mut chars = pn.chars();
    let person = match chars.next() {
        Some('1') => 1,
        Some('2') => 2,
        Some('3') => 3,
        _ => return (0, 0),
    };
    let number = match chars.next() {
        Some('s') => codes::NUMBER_SING,
        Some('p') => codes::NUMBER_PLUR,
        _ => codes::NUMBER_NONE,
    };
    (person, number)
}

/// Extrait les conjugaisons finies `(mt, personne, nombre)` d'un champ
/// `infover` (ex. « imp:pre:2s;ind:pre:1s;ind:pre:3s; »).
fn parse_finite(infover: &str) -> Vec<(u8, u8, u8)> {
    let mut out = Vec::new();
    for conj in infover.split(';') {
        if conj.is_empty() {
            continue;
        }
        let parts: Vec<&str> = conj.split(':').collect();
        if parts.len() < 3 {
            continue; // inf, par:pas… → non fini
        }
        let mt = mood_tense_code(parts[0], parts[1]);
        if mt == 0 {
            continue;
        }
        let (person, number) = parse_person_number(parts[2]);
        if person == 0 {
            continue;
        }
        let entry = (mt, person, number);
        if !out.contains(&entry) {
            out.push(entry);
        }
    }
    out
}

/// Traduit la catégorie `cgram` de Lexique vers un code de catégorie Hugo.
fn map_category(cgram: &str) -> Option<u8> {
    use codes::*;
    let cat = match cgram {
        "NOM" => CAT_NOUN,
        "VER" | "AUX" => CAT_VERB,
        "ADJ" => CAT_ADJ,
        "ART:def" | "ART:ind" | "ADJ:pos" | "ADJ:dem" | "ADJ:ind" | "ADJ:int" | "ADJ:num" => {
            CAT_DET
        }
        s if s.starts_with("PRO") => CAT_PRON,
        "ADV" => CAT_ADV,
        "PRE" => CAT_PREP,
        "CON" => CAT_CONJ,
        "ONO" => CAT_INTERJ,
        _ => CAT_UNKNOWN,
    };
    if cat == CAT_UNKNOWN {
        None
    } else {
        Some(cat)
    }
}

fn map_gender(genre: &str) -> u8 {
    match genre {
        "m" => codes::GENDER_MASC,
        "f" => codes::GENDER_FEM,
        _ => codes::GENDER_NONE,
    }
}

fn map_number(nombre: &str) -> u8 {
    match nombre {
        "s" => codes::NUMBER_SING,
        "p" => codes::NUMBER_PLUR,
        _ => codes::NUMBER_NONE,
    }
}

/// Charge la liste curée d'override de genre (`gender-overrides.tsv`) : lemme →
/// code de genre. Voir le tool `patch-morpho-gender` et le fichier lui-même pour
/// la règle d'inclusion (uniquement des noms **certainement mono-genre**, jamais
/// d'épicène ni de bi-genre). Comble les noms laissés sans genre par Lexique383.
fn load_gender_overrides(path: &str) -> Result<BTreeMap<String, u8>, Box<dyn std::error::Error>> {
    let mut map = BTreeMap::new();
    for (n, line) in std::fs::read_to_string(path)?.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.split('\t');
        let gender = match (cols.next(), cols.next().map(str::trim)) {
            (Some(lemma), Some("m")) if !lemma.is_empty() => (lemma, codes::GENDER_MASC),
            (Some(lemma), Some("f")) if !lemma.is_empty() => (lemma, codes::GENDER_FEM),
            _ => return Err(format!("override ligne {} invalide : « {line} »", n + 1).into()),
        };
        map.insert(gender.0.to_lowercase(), gender.1);
    }
    Ok(map)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if !(3..=4).contains(&args.len()) {
        eprintln!(
            "Usage : compile-morpho <Lexique383.tsv> <base_de_sortie> [gender-overrides.tsv]"
        );
        return Err("arguments invalides".into());
    }
    let src = PathBuf::from(&args[1]);
    let base = &args[2];
    let gender_overrides = match args.get(3) {
        Some(path) => load_gender_overrides(path)?,
        None => BTreeMap::new(),
    };
    let fst_path = PathBuf::from(format!("{base}.fst"));
    let bin_path = PathBuf::from(format!("{base}.bin"));
    let freq_path = PathBuf::from(format!("{base}.freq.fst"));

    eprintln!("Lecture du lexique : {}", src.display());
    let reader = BufReader::new(File::open(&src)?);

    // Agrégation : forme (minuscule) -> ensemble trié d'analyses distinctes.
    let mut lexicon: BTreeMap<String, Vec<Analysis>> = BTreeMap::new();
    // Fréquence (films, ×100, max sur les lignes d'une même forme).
    let mut frequency: BTreeMap<String, u64> = BTreeMap::new();
    let mut rows = 0usize;

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if i == 0 {
            continue; // en-tête
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 6 {
            continue;
        }
        let ortho = cols[0];
        // On ignore les locutions (espaces) et les formes élidées (apostrophe),
        // que le tokenizer de Hugo ne recherche pas telles quelles.
        if ortho.is_empty() || ortho.contains(' ') || ortho.contains('\'') {
            continue;
        }
        let Some(cat) = map_category(cols[3]) else {
            continue;
        };
        let lemma = cols[2].to_string();
        rows += 1;
        let key = ortho.to_lowercase();

        // Analyses à ajouter : pour un verbe, on développe les conjugaisons
        // finies (personne/nombre via infover) ; sinon une seule analyse.
        let mut to_add: Vec<Analysis> = Vec::new();
        if cat == codes::CAT_VERB {
            let finite = parse_finite(cols.get(10).copied().unwrap_or(""));
            if finite.is_empty() {
                // Forme non finie (infinitif, participe…) : analyse verbale
                // générique, conservée pour la reconnaissance de catégorie.
                to_add.push(Analysis {
                    cat,
                    gender: map_gender(cols[4]),
                    number: map_number(cols[5]),
                    person: 0,
                    mt: 0,
                    lemma: lemma.clone(),
                });
            } else {
                for (mt, person, number) in finite {
                    to_add.push(Analysis {
                        cat,
                        gender: codes::GENDER_NONE,
                        number,
                        person,
                        mt,
                        lemma: lemma.clone(),
                    });
                }
            }
        } else {
            to_add.push(Analysis {
                cat,
                gender: map_gender(cols[4]),
                number: map_number(cols[5]),
                person: 0,
                mt: 0,
                lemma,
            });
        }

        // Fréquence dans les livres (colonne freqlivres), en occurrences/million.
        // Préférée à freqfilms2 (oral, biais colloquial) pour un correcteur de
        // texte écrit.
        if let Some(freq) = cols.get(9).and_then(|s| s.parse::<f64>().ok()) {
            let scaled = (freq * 100.0).round().max(0.0) as u64;
            let slot = frequency.entry(key.clone()).or_insert(0);
            *slot = (*slot).max(scaled);
        }

        let entry = lexicon.entry(key).or_default();
        for analysis in to_add {
            if !entry.contains(&analysis) {
                entry.push(analysis);
            }
        }
    }

    eprintln!(
        "  {rows} lignes retenues → {} formes uniques",
        lexicon.len()
    );

    // Écriture du blob + construction du FST.
    eprintln!("Écriture : {} / {}", fst_path.display(), bin_path.display());
    if let Some(parent) = fst_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut blob: Vec<u8> = Vec::new();
    let mut fst_builder = fst::MapBuilder::new(BufWriter::new(File::create(&fst_path)?))?;
    let mut gender_filled = 0usize;

    for (form, mut analyses) in lexicon {
        // Override de genre : on comble les analyses **nominales** sans genre dont
        // le lemme figure dans la liste curée (jamais d'écrasement d'un genre connu).
        for a in &mut analyses {
            if a.cat == codes::CAT_NOUN && a.gender == codes::GENDER_NONE {
                if let Some(&g) = gender_overrides.get(&a.lemma.to_lowercase()) {
                    a.gender = g;
                    gender_filled += 1;
                }
            }
        }
        analyses.sort();
        let offset = blob.len() as u64;
        let count = analyses.len().min(255) as u64;
        for a in analyses.iter().take(255) {
            let lemma_bytes = a.lemma.as_bytes();
            let lemma_len = lemma_bytes.len().min(255) as u8;
            blob.push(a.cat);
            blob.push(a.gender);
            blob.push(a.number);
            blob.push(a.person);
            blob.push(a.mt);
            blob.push(lemma_len);
            blob.extend_from_slice(&lemma_bytes[..lemma_len as usize]);
        }
        let value = (offset << 8) | count;
        fst_builder.insert(form.as_bytes(), value)?;
    }

    fst_builder.finish()?;
    let mut bin = BufWriter::new(File::create(&bin_path)?);
    bin.write_all(&blob)?;
    bin.flush()?;
    if !gender_overrides.is_empty() {
        eprintln!(
            "  override de genre : {gender_filled} analyses comblées ({} lemmes)",
            gender_overrides.len()
        );
    }

    // FST de fréquences : forme → fréquence (×100). On n'inscrit que les formes
    // de fréquence non nulle pour limiter la taille.
    let mut freq_builder = fst::MapBuilder::new(BufWriter::new(File::create(&freq_path)?))?;
    let mut freq_forms = 0usize;
    for (form, freq) in &frequency {
        if *freq > 0 {
            freq_builder.insert(form.as_bytes(), *freq)?;
            freq_forms += 1;
        }
    }
    freq_builder.finish()?;

    let fst_size = std::fs::metadata(&fst_path)?.len();
    let bin_size = std::fs::metadata(&bin_path)?.len();
    let freq_size = std::fs::metadata(&freq_path)?.len();
    eprintln!(
        "Terminé : FST {:.1} Mo + blob {:.1} Mo + fréquences {:.1} Mo ({freq_forms} formes)",
        fst_size as f64 / 1_048_576.0,
        bin_size as f64 / 1_048_576.0,
        freq_size as f64 / 1_048_576.0,
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
