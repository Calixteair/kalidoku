# Built-in predicate families

The kalidoku engine ships with a fixed set of predicate families. A domain pack
references a family by name (the `family` field of `predicates.json`) and provides
a per-family parameter shape and a localisable label.

## Adding a new family

1. Create a sibling file `core/src/predicate/families/<my_family>.rs`.
2. Implement `Predicate` on a concrete struct holding any cached state derived
   from the parameter (precomputed regexes, parsed thresholds, etc.).
3. Implement `PredicateFactory` on a unit struct named `Factory`.
4. Register it in `families/mod.rs::register_defaults`.
5. Update the `enum` of allowed families in `contracts/entity-schema.json`
   (separate PR — touching `contracts/` is cross-scope).

Family identifiers must be `snake_case`, stable, and never reused for a different
behaviour.

## Reference

| family               | param shape                                       | semantics                                                                |
|----------------------|---------------------------------------------------|--------------------------------------------------------------------------|
| `ends_with`          | `"S"`                                             | normalised name ends with the given letter                               |
| `starts_with`        | `"C"`                                             | normalised name starts with the given letter                             |
| `contains_letter`    | `"e"`                                             | normalised name contains the given letter                                |
| `name_length_max`    | `7`                                               | normalised name (no spaces) is at most `n` characters                    |
| `name_length_min`    | `4`                                               | normalised name is at least `n` characters                               |
| `name_length_eq`     | `6`                                               | normalised name has exactly `n` characters                               |
| `on_attr_eq`         | `{ "attr": "in_paris", "value": true }`           | scalar attribute equals the given value                                  |
| `on_attr_in_set`     | `{ "attr": "lines", "values": ["1","4","14"] }`   | str/str_list attribute intersects the value set                          |
| `on_attr_contains`   | `{ "attr": "lines", "value": "5" }`               | str_list attribute contains the given value                              |
| `within_km`          | `{ "attr": "geo", "from": {...}, "km": 3.0 }`     | geo attribute within `km` of the anchor (haversine, geoutils)            |
| `numeric_gte`        | `{ "attr": "arrondissement", "n": 5 }`            | numeric attribute ≥ `n`                                                  |
| `numeric_lte`        | `{ "attr": "arrondissement", "n": 8 }`            | numeric attribute ≤ `n`                                                  |
| `numeric_between`    | `{ "attr": "arrondissement", "min": 1, "max": 8 }`| numeric attribute in `[min, max]`                                        |

`normalised name` follows `core::normalize::normalize`: NFD-stripped diacritics,
lowercase, single-space punctuation. The same function is used at index time
and at query time so the engine and the autocomplete agree on letters.
