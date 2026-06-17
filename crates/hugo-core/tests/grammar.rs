//! Tests d'intégration des règles grammaticales sur le pipeline complet
//! ([`Checker`]), avec un petit corpus annoté et un contrôle de performance.

use std::time::Instant;

use hugo_core::Checker;

/// Identifiants des règles grammaticales (par opposition à l'orthographe et à
/// la capitalisation, hors champ de ce corpus).
const GRAMMAR_RULES: &[&str] = &[
    "determiner_noun_agreement",
    "subject_verb_agreement",
    "attribute_adjective_agreement",
    "epithet_adjective_agreement",
    "tout_agreement",
    "homophone",
    "confusion_a_a",
    "confusion_ce_se",
    "confusion_ou",
    "confusion_la",
    "confusion_leur",
    "confusion_peu",
    "confusion_quel",
    "confusion_quand",
    "confusion_sans",
    "confusion_terminaison",
    "past_participle_avoir",
    "passive_participle",
];

/// Suggestions grammaticales d'un texte (orthographe/capitalisation exclues).
fn grammar_suggestions(checker: &Checker, text: &str) -> Vec<hugo_core::Suggestion> {
    checker
        .check(text)
        .into_iter()
        .filter(|s| GRAMMAR_RULES.contains(&s.rule_id))
        .collect()
}

/// Un cas fautif : `(texte, rule_id attendu, fragment de remplacement attendu)`.
const INCORRECT: &[(&str, &str, &str)] = &[
    // Accord déterminant–nom.
    ("un table", "determiner_noun_agreement", "une"),
    ("les chat", "determiner_noun_agreement", "le"),
    ("du table", "determiner_noun_agreement", "de la"),
    ("aux chat", "determiner_noun_agreement", "au"),
    // Accord sujet–verbe, sujet pronominal.
    ("ils mange", "subject_verb_agreement", "mangent"),
    ("tu mange", "subject_verb_agreement", "manges"),
    // Accord sujet–verbe, sujet nominal.
    ("les chats mange", "subject_verb_agreement", "mangent"),
    ("mes amis arrive", "subject_verb_agreement", "arrivent"),
    // Accord sujet–verbe, sujet coordonné.
    ("Pierre et Marie mange", "subject_verb_agreement", "mangent"),
    ("le chat et le chien mange", "subject_verb_agreement", "mangent"),
    ("toi et moi est là", "subject_verb_agreement", "sommes"),
    // Accord de l'attribut.
    ("elle est content", "attribute_adjective_agreement", "contente"),
    ("ils sont content", "attribute_adjective_agreement", "contents"),
    // Accord de l'adjectif épithète.
    ("les chats noir", "epithet_adjective_agreement", "noirs"),
    ("les petit chats", "epithet_adjective_agreement", "petits"),
    ("un beau table", "epithet_adjective_agreement", "belle"),
    // Participe passé avec être.
    ("elle est parti", "attribute_adjective_agreement", "partie"),
    ("ils sont allé", "attribute_adjective_agreement", "allés"),
    // Accord de « tout ».
    ("toute les jours", "tout_agreement", "tous"),
    ("tout les semaines", "tout_agreement", "toutes"),
    // Confusion a/à.
    ("il va a Paris", "confusion_a_a", "à"),
    ("il à faim", "confusion_a_a", "a"),
    // Homophones.
    ("ils on mangé", "homophone", "ont"),
    ("ils son partis", "homophone", "sont"),
    // Confusion ce/se, c'est/s'est.
    ("il ce lève", "confusion_ce_se", "se"),
    ("il aime se chien", "confusion_ce_se", "ce"),
    ("il c'est trompé", "confusion_ce_se", "s'"),
    ("s'est magnifique", "confusion_ce_se", "c'"),
    // Confusion ou/où, la/là/l'a, leur/leurs, peu/peut/peux (tranche 3).
    ("le jour ou je suis né", "confusion_ou", "où"),
    ("là maison est belle", "confusion_la", "la"),
    ("il la mangé", "confusion_la", "l'a"),
    ("leur livres sont neufs", "confusion_leur", "leurs"),
    ("je leurs parle", "confusion_leur", "leur"),
    ("il peu marcher", "confusion_peu", "peut"),
    ("un peut de sel", "confusion_peu", "peu"),
    // Confusion quel/qu'elle, quand/quant, sans/s'en (tranche 4).
    ("je crois quelle vient demain", "confusion_quel", "qu'elle"),
    ("je me demande qu'elle heure il est", "confusion_quel", "quelle"),
    ("quand à moi je reste", "confusion_quand", "quant"),
    ("quant il pleut je lis", "confusion_quand", "quand"),
    ("il sans va sans rien dire", "confusion_sans", "s'en"),
    ("il a réussi s'en effort", "confusion_sans", "sans"),
    // Confusion des terminaisons -er/-é/-ez (tranche 5).
    ("il a manger une pomme", "confusion_terminaison", "mangé"),
    ("il commence à mangé", "confusion_terminaison", "manger"),
    ("vous manger trop de sucre", "confusion_terminaison", "mangez"),
    // Participe passé avec avoir + COD antéposé (que/COD relatif).
    ("les lettres qu'il a écrit", "past_participle_avoir", "écrites"),
    ("les livres que j'ai lu", "past_participle_avoir", "lus"),
    ("les décisions qu'on a prit", "past_participle_avoir", "prises"),
    // Voix passive.
    ("la lettre a été écris", "passive_participle", "écrite"),
];

/// Phrases correctes : aucune suggestion grammaticale ne doit apparaître.
const CORRECT: &[&str] = &[
    "une table",
    "le chat mange",
    "les chats mangent",
    "du pain",
    "au chat",
    "ils mangent",
    "nous mangeons",
    "les chats noirs dorment",
    "un beau chat noir",
    "une belle grande maison",
    "les petits chats blancs",
    "Pierre et Marie mangent",
    "le chat et le chien dorment",
    "toi et moi sommes là",
    "elle est contente",
    "ils sont contents",
    "il va à Paris",
    "il a faim",
    "ils ont mangé",
    "son chat dort",
    "il se lève",
    "ce livre est lourd",
    "c'est une bonne idée",
    "il s'est levé tôt",
    "ces livres sont neufs",
    "ses amis sont venus",
    "je vois les chats",
    "Jean dort et Marie mange",
    "elle est partie",
    "ils sont allés",
    "il est venu",
    "tout le monde est parti",
    "toutes les semaines",
    "toute la journée",
    "le jour où je suis né",
    "le jour ou la nuit",
    "la maison est belle",
    "il l'a vu hier",
    "il la voit chaque jour",
    "leur livre est neuf",
    "leurs livres sont neufs",
    "je leur parle",
    "il peut marcher",
    "un peu de sel",
    "quelle heure est-il",
    "je crois qu'elle vient",
    "quand il pleut je lis",
    "quant à moi je reste",
    "il s'en va sans rien dire",
    "il réussit sans effort",
    // Terminaisons -er/-é/-ez (tranche 5) : usages corrects.
    "il a mangé une pomme",
    "il commence à manger",
    "je veux manger ce soir",
    "vous mangez trop de sucre",
    "il veut vous voir demain",
    "le saumon fumé est délicieux",
    // Robustesse de l'identification du sujet (consommation du POS) : ces
    // phrases produisaient des faux positifs d'accord sujet–verbe / d'attribut
    // avant que `conjugation` et `attribute` ne consomment les tags CRF.

    "le problème qui nous a été posé",
    "la démultiplication des usages digitaux alla de pair avec les risques",
    "la nécessité grandissante de pouvoir redéployer des postes compromis",
    "l'outil propulsé dans une nouvelle ère fut nommé",
    // Proposition participiale : « père » (objet du participe présent
    // « fatiguant ») n'est pas le sujet de « sont ».
    "les filles fatiguant leur père sont fatigantes",
    // Homographe préposition/adjectif : « sur » (ADP) n'est pas un attribut à
    // accorder en « sure ».
    "elle est sur le côté",
    "la table est sur le côté",
    // Audit faux positifs (corpus correct). Noms invariables en nombre :
    "au cours de la journée",
    "il a fait cela à la fois pour ses amis",
    "cet homme est partout",
    "cet article est intéressant",
    // « N de N (de N) ADJ » : rattachement de l'adjectif ambigu → abstention.
    "des outils de travail professionnels",
    "une marge nette d'intérêts solide",
    // « vous »/« nous » clitiques objets, pas sujets :
    "notre expérience vous assure la réalisation",
    "arrête la lecture si cela vous suffit",
    // Attribut nominal et superlatif :
    "les données sont la partie la plus visible",
    // Terminaison : nom homographe d'un verbe après déterminant/préposition :
    "il faut enrichir la base de données",
];

#[test]
fn incorrect_sentences_are_flagged() {
    let checker = Checker::new();
    for &(text, rule_id, expected) in INCORRECT {
        let found = grammar_suggestions(&checker, text);
        let hit = found
            .iter()
            .any(|s| s.rule_id == rule_id && s.replacements.iter().any(|r| r == expected));
        assert!(
            hit,
            "« {text} » : attendu {rule_id} → « {expected} », obtenu {:?}",
            found
                .iter()
                .map(|s| (s.rule_id, &s.replacements))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn correct_sentences_yield_no_grammar_suggestion() {
    let checker = Checker::new();
    for &text in CORRECT {
        let found = grammar_suggestions(&checker, text);
        assert!(
            found.is_empty(),
            "faux positif sur « {text} » : {:?}",
            found
                .iter()
                .map(|s| (s.rule_id, &s.replacements))
                .collect::<Vec<_>>()
        );
    }
}

/// Contrôle de performance : après chauffe (chargement paresseux des index
/// morphologiques), une phrase de ~20 mots doit se vérifier bien en deçà du
/// budget. Le seuil est volontairement large pour ne pas être instable en CI ;
/// la cible réelle de la feuille de route est <5 ms.
#[test]
fn performance_is_within_budget() {
    let checker = Checker::new();
    let sentence = "Le petit chat noir et le grand chien blanc mange souvent des croquettes \
                    dans la cuisine près de la fenêtre ouverte.";

    // Chauffe : force la construction des index de conjugaison/déclinaison.
    for _ in 0..5 {
        let _ = checker.check(sentence);
    }

    let iterations = 200;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = checker.check(sentence);
    }
    let per_sentence = start.elapsed() / iterations;

    println!("Temps moyen par phrase : {per_sentence:?}");
    assert!(
        per_sentence.as_millis() < 50,
        "trop lent : {per_sentence:?} par phrase"
    );
}
