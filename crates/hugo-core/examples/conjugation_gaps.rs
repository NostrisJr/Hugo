//! Audit exhaustif des lacunes de génération conjugaison dans le Lefff.
//!
//! Usage : `cargo run -p hugo-core --example conjugation_gaps`
//!
//! Pour chaque lemme verbal connu du Lefff, vérifie si toutes les formes
//! standard (indicatif, subjonctif, conditionnel, impératif) sont générables
//! via `morpho::conjugate`. Affiche uniquement les lacunes.

use hugo_core::morpho::{self, MoodTense, Number, Person};

fn main() {
    // Toutes les combinaisons (mode/temps, personne, nombre) attendues pour un verbe.
    let combos: Vec<(MoodTense, Person, Number)> = {
        let mut v = Vec::new();
        for &mt in &[
            MoodTense::IndicativePresent,
            MoodTense::IndicativeImperfect,
            MoodTense::IndicativeFuture,
            MoodTense::IndicativePast,
            MoodTense::ConditionalPresent,
            MoodTense::SubjunctivePresent,
            MoodTense::SubjunctiveImperfect,
        ] {
            for &p in &[Person::First, Person::Second, Person::Third] {
                for &n in &[Number::Singular, Number::Plural] {
                    v.push((mt, p, n));
                }
            }
        }
        // Impératif : seulement 2sg, 1pl, 2pl
        v.push((MoodTense::Imperative, Person::Second, Number::Singular));
        v.push((MoodTense::Imperative, Person::First, Number::Plural));
        v.push((MoodTense::Imperative, Person::Second, Number::Plural));
        v
    };

    // Reconstruire la liste des lemmes verbaux depuis le lexique :
    // un lemme verbal est un lemme pour lequel conjugate(lemme, IndPres, 1sg, Sing) != None
    // OU qui possède au moins une forme conjuguée dans le Lefff.
    // On passe par un ensemble de formes verbales connues pour extraire les lemmes.
    let mut lemmas: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // Heuristique : on cherche les lemmes via les formes dans les entrées verbales finies.
    // On sonde directement les formes verbales de verbes courants pour amorcer.
    // En pratique, on extrait les lemmes depuis verb_forms des formes du Lefff.
    // Comme on ne peut pas itérer le FST directement depuis l'API publique,
    // on utilise un ensemble de formes représentatives par famille.

    // Familles d'irréguliers à auditer (lemmes canoniques + dérivés courants).
    let seed_lemmas = [
        // Auxiliaires
        "être", "avoir",
        // Aller
        "aller",
        // Faire et composés
        "faire", "défaire", "refaire", "satisfaire", "contrefaire",
        // Dire et composés
        "dire", "redire", "contredire", "maudire", "médire", "prédire",
        // Prendre et composés
        "prendre", "apprendre", "comprendre", "entreprendre", "reprendre",
        "surprendre", "désapprendre",
        // Venir et composés
        "venir", "devenir", "revenir", "convenir", "parvenir", "prévenir",
        "provenir", "survenir", "intervenir",
        // Tenir et composés
        "tenir", "appartenir", "contenir", "détenir", "entretenir", "maintenir",
        "obtenir", "retenir", "soutenir",
        // Voir et composés
        "voir", "revoir", "prévoir", "pourvoir", "entrevoir",
        // Pouvoir / Vouloir / Devoir / Savoir
        "pouvoir", "vouloir", "devoir", "savoir", "falloir",
        // Valoir
        "valoir", "prévaloir", "équivaloir",
        // Asseoir / Assoir
        "asseoir", "assoir", "rasseoir", "rassoir",
        // Mouvoir et composés
        "mouvoir", "émouvoir", "promouvoir",
        // Recevoir et composés
        "recevoir", "apercevoir", "concevoir", "décevoir", "percevoir",
        // Mettre et composés
        "mettre", "admettre", "commettre", "compromettre", "démettre",
        "émettre", "omettre", "permettre", "promettre", "remettre",
        "soumettre", "transmettre",
        // Battre et composés
        "battre", "abattre", "combattre", "débattre", "rebattre",
        // Suivre et composés
        "suivre", "poursuivre",
        // Vivre et composés
        "vivre", "revivre", "survivre",
        // Écrire et composés
        "écrire", "décrire", "inscrire", "prescrire", "proscrire", "souscrire",
        "transcrire",
        // Lire et composés
        "lire", "élire", "relire",
        // Conduire et composés
        "conduire", "déduire", "produire", "réduire", "séduire", "traduire",
        "induire", "introduire", "construire", "détruire", "instruire",
        // Naître / Croître
        "naître", "renaître", "croître",
        // Connaître et composés
        "connaître", "méconnaître", "reconnaître", "paraître", "apparaître",
        "disparaître", "transparaître",
        // Partir, sortir, sentir
        "partir", "sortir", "sentir", "mentir", "consentir", "repartir",
        "ressortir",
        // Dormir / Servir
        "dormir", "servir", "resservir",
        // Courir et composés
        "courir", "accourir", "concourir", "discourir", "parcourir", "recourir",
        "secourir",
        // Mourir
        "mourir",
        // Ouvrir et composés
        "ouvrir", "couvrir", "découvrir", "recouvrir", "offrir", "souffrir",
        // Cueillir
        "cueillir", "accueillir", "recueillir",
        // Acquérir et composés
        "acquérir", "conquérir", "requérir",
        // Fuir et composés
        "fuir", "s'enfuir",
        // Plaire et composés
        "plaire", "déplaire", "complaire", "taire",
        // Braire / Traire et composés
        "traire", "abstraire", "distraire", "extraire", "soustraire",
        // Résoudre et composés
        "résoudre", "absoudre", "dissoudre",
        // Craindre, peindre, teindre et composés
        "craindre", "plaindre", "contraindre", "peindre", "dépeindre",
        "repeindre", "teindre", "éteindre", "atteindre", "ceindre",
        "feindre", "joindre", "adjoindre", "conjoindre", "rejoindre",
        // Rompre et composés
        "rompre", "corrompre", "interrompre",
        // Vaincre et composés
        "vaincre", "convaincre",
        // Boire
        "boire",
        // Clore
        "clore",
        // Moudre
        "moudre",
        // Coudre
        "coudre",
        // Vêtir
        "vêtir", "revêtir",
        // Saillir / Faillir
        "faillir", "saillir",
    ];

    for lemma in &seed_lemmas {
        lemmas.insert(lemma.to_string());
    }

    // Pour les verbes dont le lemme n'est pas dans verb_forms("infinitif")
    // (ex. lemmes dérivés qui n'ont pas d'entrée directe), on détecte leur
    // présence en sondant verb_forms de leurs formes courtes.
    let sonde_forms = [
        "suis", "es", "est", "sommes", "êtes", "sont",
        "ai", "as", "a", "avons", "avez", "ont",
        "vais", "vas", "va", "allons", "allez", "vont",
    ];
    for f in &sonde_forms {
        for vf in morpho::verb_forms(f) {
            lemmas.insert(vf.lemma);
        }
    }

    println!("=== Audit lacunes conjugaison ===");
    println!("Verbes audités : {}", lemmas.len());
    println!();

    // Modes/temps avec leur impératif exclu pour certains verbes.
    // On signale une lacune seulement si le verbe possède AU MOINS UNE forme pour ce mode/temps.
    // Verbes défectifs connus : on ne signale pas leurs lacunes structurelles.
    let defective: std::collections::HashSet<&str> = [
        "falloir",  // impersonnel : 3sg uniquement en français standard
    ].into_iter().collect();
    let mut total_gaps = 0;

    for lemma in &lemmas {
        if defective.contains(lemma.as_str()) { continue; }
        let mut gaps: Vec<String> = Vec::new();

        for &(mt, p, n) in &combos {
            let result = morpho::conjugate(lemma, mt, p, n);
            if result.is_none() {
                // Vérifier si ce mode/temps est attendu pour ce verbe
                // (certains verbes sont défectifs par nature).
                // On considère qu'un verbe doit avoir la forme si une autre forme du même
                // mode/temps existe pour lui.
                let other_exists = combos.iter().any(|&(mt2, p2, n2)| {
                    mt2 == mt && (p2, n2) != (p, n) && morpho::conjugate(lemma, mt2, p2, n2).is_some()
                });
                if other_exists {
                    gaps.push(format!(
                        "{}/{:?}/{:?}",
                        match mt {
                            MoodTense::IndicativePresent => "IndPres",
                            MoodTense::IndicativeImperfect => "IndImperf",
                            MoodTense::IndicativeFuture => "IndFut",
                            MoodTense::IndicativePast => "PasséSimple",
                            MoodTense::ConditionalPresent => "Cond",
                            MoodTense::SubjunctivePresent => "SubjPres",
                            MoodTense::SubjunctiveImperfect => "SubjImperf",
                            MoodTense::Imperative => "Impér",
                        },
                        p, n
                    ));
                }
            }
        }

        if !gaps.is_empty() {
            println!("  {lemma}:");
            for g in &gaps {
                total_gaps += 1;
                println!("    - {g}");
            }
        }
    }

    println!();
    println!("Total lacunes : {total_gaps}");

    // Audit double conjugaison (asseoir/assoir)
    println!();
    println!("=== Double conjugaison (asseoir / assoir) ===");
    for lemma in &["asseoir", "assoir", "rasseoir", "rassoir"] {
        println!("{lemma}:");
        let combos_check = [
            (MoodTense::IndicativePresent, Person::First, Number::Singular),
            (MoodTense::IndicativePresent, Person::Third, Number::Singular),
            (MoodTense::IndicativePresent, Person::First, Number::Plural),
            (MoodTense::IndicativePresent, Person::Third, Number::Plural),
        ];
        for (mt, p, n) in combos_check {
            let r = morpho::conjugate(lemma, mt, p, n);
            println!("  {:?}/{:?} = {:?}", p, n, r);
        }
    }
}
