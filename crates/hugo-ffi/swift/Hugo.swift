// Hugo.swift — surcouche Swift idiomatique au-dessus de l'API C (module CHugo).
//
// Dépend du XCFramework produit par `scripts/build-xcframework.sh`, qui expose
// le module C `CHugo` (voir include/module.modulemap).
//
// Exemple :
//
//     let checker = HugoChecker()
//     for s in checker.check("il il va a Paris") {
//         print(s.byteRange, s.message, s.replacements)
//     }

import CHugo
import Foundation

/// Une suggestion de correction renvoyée par ``HugoChecker``.
public struct HugoSuggestion: Sendable, Equatable {
    /// Plage d'**octets** UTF-8 dans le texte source (demi-ouverte).
    public let byteRange: Range<Int>
    /// Message explicatif (en français).
    public let message: String
    /// Corrections proposées, triées de la plus à la moins pertinente.
    /// Une chaîne vide signifie « supprimer le fragment ».
    public let replacements: [String]
    /// Identifiant stable de la règle (ex. `"homophone"`).
    public let ruleId: String
}

/// Correcteur orthographique et grammatical français, entièrement local.
///
/// L'instance détient des ressources natives ; elle est libérée
/// automatiquement (`deinit`). `HugoChecker` n'est pas thread-safe : utilisez
/// une instance par thread, ou sérialisez les appels.
public final class HugoChecker {
    private let handle: OpaquePointer?

    /// Construit un correcteur (charge les dictionnaires embarqués).
    public init() {
        handle = hugo_checker_new()
    }

    deinit {
        hugo_checker_free(handle)
    }

    /// Vérifie `text` et renvoie les suggestions.
    public func check(_ text: String) -> [HugoSuggestion] {
        let results = text.withCString { hugo_checker_check(handle, $0) }
        defer { hugo_free_results(results) }

        guard results.len > 0, let base = results.suggestions else { return [] }

        return (0..<results.len).map { i in
            let s = base[i]
            var replacements: [String] = []
            if s.replacements_len > 0, let reps = s.replacements {
                replacements.reserveCapacity(s.replacements_len)
                for j in 0..<s.replacements_len {
                    if let cstr = reps[j] {
                        replacements.append(String(cString: cstr))
                    }
                }
            }
            return HugoSuggestion(
                byteRange: Int(s.start)..<Int(s.end),
                message: s.message.map { String(cString: $0) } ?? "",
                replacements: replacements,
                ruleId: s.rule_id.map { String(cString: $0) } ?? ""
            )
        }
    }
}

public extension HugoSuggestion {
    /// Convertit la plage d'octets en `Range<String.Index>` sur `text`, ou
    /// `nil` si les bornes ne tombent pas sur des frontières de caractères.
    func stringRange(in text: String) -> Range<String.Index>? {
        let utf8 = text.utf8
        guard
            let lo = utf8.index(utf8.startIndex, offsetBy: byteRange.lowerBound, limitedBy: utf8.endIndex),
            let hi = utf8.index(utf8.startIndex, offsetBy: byteRange.upperBound, limitedBy: utf8.endIndex),
            let start = lo.samePosition(in: text),
            let end = hi.samePosition(in: text)
        else { return nil }
        return start..<end
    }
}
