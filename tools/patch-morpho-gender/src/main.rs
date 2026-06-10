//! Patch de genre du blob morphologique de Hugo.
//!
//! Le lexique source (Lexique383) laisse le **genre vide** pour ~5 % des noms.
//! La plupart sont **légitimement** sans genre fixe (épicènes : « un/une
//! camarade » ; bi-genres : « le/la tour, le/la livre »), mais une minorité sont
//! de **vrais trous** mono-genre (« maison », « voiture », « main »…).
//!
//! Ce tool comble ces trous à partir d'une **liste curée** (`gender-overrides.tsv`,
//! nos propres données — uniquement des noms **certainement mono-genre**, jamais
//! d'épicène ni de bi-genre) :
//!
//! - il ne touche **que** les analyses de catégorie **nom** dont le genre est
//!   actuellement `None` (octet 0) — un genre déjà renseigné n'est jamais écrasé ;
//! - l'appariement se fait par **lemme**, donc singulier et pluriel sont corrigés
//!   d'un coup ;
//! - seul l'**octet de genre** est réécrit en place : les offsets du FST restent
//!   valides, le `.fst` est inchangé.
//!
//! La même liste est honorée par [`compile-morpho`] lors d'une recompilation
//! depuis la source ; ce tool sert à appliquer l'override à l'asset **déjà
//! compilé** (la source n'étant pas vendue dans le dépôt).
//!
//! Usage : `patch-morpho-gender <morpho.fst> <morpho.bin> <gender-overrides.tsv>`

use std::collections::HashMap;
use std::fs;

use fst::{Map, Streamer};

const CAT_NOUN: u8 = 1;
const GENDER_NONE: u8 = 0;
const GENDER_MASC: u8 = 1;
const GENDER_FEM: u8 = 2;

fn parse_gender(code: &str) -> Option<u8> {
    match code.trim() {
        "m" => Some(GENDER_MASC),
        "f" => Some(GENDER_FEM),
        _ => None,
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage : patch-morpho-gender <morpho.fst> <morpho.bin> <gender-overrides.tsv>");
        return Err("arguments invalides".into());
    }
    let (fst_path, bin_path, tsv_path) = (&args[1], &args[2], &args[3]);

    // Liste curée : lemme -> genre (m/f). Lignes vides et commentaires « # » ignorés.
    let mut overrides: HashMap<String, u8> = HashMap::new();
    for (n, line) in fs::read_to_string(tsv_path)?.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.split('\t');
        match (cols.next(), cols.next().and_then(parse_gender)) {
            (Some(lemma), Some(g)) if !lemma.is_empty() => {
                overrides.insert(lemma.to_lowercase(), g);
            }
            _ => return Err(format!("ligne {} invalide : « {line} »", n + 1).into()),
        }
    }
    eprintln!("{} lemmes d'override chargés", overrides.len());

    let map = Map::new(fs::read(fst_path)?)?;
    let mut blob = fs::read(bin_path)?;

    let mut changed = 0usize;
    let mut matched: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut stream = map.stream();
    while let Some((_key, value)) = stream.next() {
        let offset = (value >> 8) as usize;
        let count = (value & 0xff) as usize;
        let mut p = offset;
        for _ in 0..count {
            let cat = blob[p];
            let gender = blob[p + 1];
            let len = blob[p + 5] as usize;
            let lemma = String::from_utf8_lossy(&blob[p + 6..p + 6 + len]).into_owned();
            if cat == CAT_NOUN && gender == GENDER_NONE {
                if let Some(&g) = overrides.get(&lemma) {
                    blob[p + 1] = g;
                    changed += 1;
                    matched.insert(lemma);
                }
            }
            p += 6 + len;
        }
    }

    fs::write(bin_path, &blob)?;
    eprintln!(
        "{changed} analyses corrigées ({} lemmes effectifs) → {bin_path}",
        matched.len()
    );

    // Lemmes de la liste qui n'ont rien corrigé (déjà genrés, ou absents du
    // lexique) : utile pour élaguer le fichier d'override.
    let ineffective: Vec<&String> = overrides.keys().filter(|l| !matched.contains(*l)).collect();
    if !ineffective.is_empty() {
        let mut sorted = ineffective;
        sorted.sort();
        eprintln!("{} lemmes sans effet : {:?}", sorted.len(), sorted);
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("erreur : {e}");
        std::process::exit(1);
    }
}
