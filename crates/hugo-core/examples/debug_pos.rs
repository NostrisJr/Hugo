use hugo_core::pos;
use hugo_core::rules::detached_appositive::DetachedAppositive;
use hugo_core::rules::attribute::AttributeAdjectiveAgreement;
use hugo_core::rules::Rule;
use hugo_core::tokenizer::tokenize;

fn check(phrase: &str) {
    let tokens = tokenize(phrase);
    let tags = pos::tag(&tokens);
    let s1 = DetachedAppositive.check_tagged(&tokens, &tags);
    let s2 = AttributeAdjectiveAgreement.check_tagged(&tokens, &tags);
    println!("\n=== {} ===", phrase);
    for s in s1.iter().chain(s2.iter()) {
        println!("  CORRECTION: {:?}", s.replacements);
        println!("  MESSAGE:    {}", s.message);
    }
    if s1.is_empty() && s2.is_empty() {
        println!("  (aucune suggestion)");
    }
}

fn main() {
    check("Sous le vent sera déployé ma voile.");
    check("Dans la brume et dans le silence, tapie sous un rocher, le garçon attendait.");
    // Régressions à vérifier
    check("Au bord du lac, endormi à l'ombre des arbres, patientaient les soldats.");
    check("Au bord du lac, endormi à l'ombre des arbres, le renard regardait les enfants.");
    check("elle est content");
    check("elle est sur le côté");
}
