/**
 * Mapping statique stations -> lignes du métro de Paris.
 *
 * Source : https://fr.wikipedia.org/wiki/Liste_des_stations_du_m%C3%A9tro_de_Paris (publique).
 * Utilisé comme fallback robuste si Overpass est down ou ne donne pas de match propre.
 *
 * La clé est le nom canonique tel qu'OSM/RATP l'écrit. Les variantes courantes (avec ou
 * sans tiret cadratin) sont normalisées par `normalizeStationName`.
 */

export function normalizeStationName(name: string): string {
  return name
    .normalize("NFD")
    .replace(/\p{Diacritic}+/gu, "")
    .toLowerCase()
    .replace(/[\u2013\u2014\u2212]/g, "-")
    .replace(/\s*-\s*/g, "-")
    .replace(/\s+/g, " ")
    .replace(/[^a-z0-9 -]/g, "")
    .trim();
}

/**
 * Mapping nom-normalisé -> lignes (str_list).
 * Couvre l'intégralité des stations en service du métro de Paris (16 lignes).
 */
export const STATION_LINES: Record<string, string[]> = {
  // Ligne 1 : La Défense -> Château de Vincennes
  "esplanade de la defense": ["1"],
  "pont de neuilly": ["1"],
  "les sablons": ["1"],
  "porte maillot": ["1"],
  "argentine": ["1"],
  "charles de gaulle-etoile": ["1", "2", "6"],
  "george v": ["1"],
  "franklin d roosevelt": ["1", "9"],
  "franklin d. roosevelt": ["1", "9"],
  "champs-elysees-clemenceau": ["1", "13"],
  "concorde": ["1", "8", "12"],
  "tuileries": ["1"],
  "palais royal-musee du louvre": ["1", "7"],
  "louvre-rivoli": ["1"],
  "chatelet": ["1", "4", "7", "11", "14"],
  "hotel de ville": ["1", "11"],
  "saint-paul": ["1"],
  "bastille": ["1", "5", "8"],
  "gare de lyon": ["1", "14"],
  "reuilly-diderot": ["1", "8"],
  "nation": ["1", "2", "6", "9"],
  "porte de vincennes": ["1"],
  "saint-mande": ["1"],
  "berault": ["1"],
  "chateau de vincennes": ["1"],

  // Ligne 2 : Porte Dauphine -> Nation
  "porte dauphine": ["2"],
  "victor hugo": ["2"],
  "ternes": ["2"],
  "courcelles": ["2"],
  "monceau": ["2"],
  "villiers": ["2", "3"],
  "rome": ["2"],
  "place de clichy": ["2", "13"],
  "blanche": ["2"],
  "pigalle": ["2", "12"],
  "anvers": ["2"],
  "barbes-rochechouart": ["2", "4"],
  "la chapelle": ["2"],
  "stalingrad": ["2", "5", "7"],
  "jaures": ["2", "5", "7bis"],
  "colonel fabien": ["2"],
  "belleville": ["2", "11"],
  "couronnes": ["2"],
  "menilmontant": ["2"],
  "pere lachaise": ["2", "3"],
  "philippe auguste": ["2"],
  "alexandre dumas": ["2"],
  "avron": ["2"],

  // Ligne 3 : Pont de Levallois - Bécon -> Gallieni
  "pont de levallois-becon": ["3"],
  "anatole france": ["3"],
  "louise michel": ["3"],
  "porte de champerret": ["3"],
  "pereire": ["3"],
  "wagram": ["3"],
  "malesherbes": ["3"],
  "europe": ["3"],
  "saint-lazare": ["3", "12", "13", "14"],
  "havre-caumartin": ["3", "9"],
  "opera": ["3", "7", "8"],
  "quatre-septembre": ["3"],
  "bourse": ["3"],
  "sentier": ["3"],
  "reaumur-sebastopol": ["3", "4"],
  "arts et metiers": ["3", "11"],
  "temple": ["3"],
  "republique": ["3", "5", "8", "9", "11"],
  "parmentier": ["3"],
  "rue saint-maur": ["3"],
  "saint-maur": ["3"],
  "gambetta": ["3"],
  "porte de bagnolet": ["3"],
  "gallieni": ["3"],

  // Ligne 3bis : Gambetta -> Porte des Lilas
  "pelleport": ["3bis"],
  "saint-fargeau": ["3bis"],
  "porte des lilas": ["3bis", "11"],

  // Ligne 4 : Porte de Clignancourt -> Bagneux - Lucie Aubrac
  "porte de clignancourt": ["4"],
  "simplon": ["4"],
  "marcadet-poissonniers": ["4", "12"],
  "chateau rouge": ["4"],
  "gare du nord": ["4", "5"],
  "gare de l'est": ["4", "5", "7"],
  "gare de lest": ["4", "5", "7"],
  "chateau d'eau": ["4"],
  "chateau deau": ["4"],
  "strasbourg-saint-denis": ["4", "8", "9"],
  "etienne marcel": ["4"],
  "les halles": ["4"],
  "cite": ["4"],
  "saint-michel": ["4"],
  "odeon": ["4", "10"],
  "saint-germain-des-pres": ["4"],
  "saint-sulpice": ["4"],
  "saint-placide": ["4"],
  "montparnasse-bienvenue": ["4", "6", "12", "13"],
  "vavin": ["4"],
  "raspail": ["4", "6"],
  "denfert-rochereau": ["4", "6"],
  "mouton-duvernet": ["4"],
  "alesia": ["4"],
  "porte d'orleans": ["4"],
  "porte dorleans": ["4"],
  "mairie de montrouge": ["4"],
  "barbara": ["4"],
  "bagneux-lucie aubrac": ["4"],

  // Ligne 5 : Bobigny - Pablo Picasso -> Place d'Italie
  "bobigny-pablo picasso": ["5"],
  "bobigny-pantin-raymond queneau": ["5"],
  "eglise de pantin": ["5"],
  "hoche": ["5"],
  "porte de pantin": ["5"],
  "ourcq": ["5"],
  "laumiere": ["5"],
  "gare du nord-magenta": ["5"],
  "jacques bonsergent": ["5"],
  "oberkampf": ["5", "9"],
  "richard-lenoir": ["5"],
  "breguet-sabin": ["5"],
  "quai de la rapee": ["5"],
  "campo-formio": ["5"],
  "place d'italie": ["5", "6", "7"],
  "place ditalie": ["5", "6", "7"],

  // Ligne 6 : Charles de Gaulle - Étoile -> Nation
  "kleber": ["6"],
  "boissiere": ["6"],
  "trocadero": ["6", "9"],
  "passy": ["6"],
  "bir-hakeim": ["6"],
  "dupleix": ["6"],
  "la motte-picquet-grenelle": ["6", "8", "10"],
  "cambronne": ["6"],
  "sevres-lecourbe": ["6"],
  "pasteur": ["6", "12"],
  "edgar quinet": ["6"],
  "saint-jacques": ["6"],
  "glaciere": ["6"],
  "corvisart": ["6"],
  "nationale": ["6"],
  "chevaleret": ["6"],
  "quai de la gare": ["6"],
  "bercy": ["6", "14"],
  "dugommier": ["6"],
  "daumesnil": ["6", "8"],
  "bel-air": ["6"],
  "picpus": ["6"],

  // Ligne 7 : La Courneuve - 8 Mai 1945 -> Villejuif - Louis Aragon / Mairie d'Ivry
  "la courneuve-8 mai 1945": ["7"],
  "fort d'aubervilliers": ["7"],
  "fort daubervilliers": ["7"],
  "aubervilliers-pantin-quatre chemins": ["7"],
  "porte de la villette": ["7"],
  "corentin cariou": ["7"],
  "crimee": ["7"],
  "riquet": ["7"],
  "louis blanc": ["7", "7bis"],
  "chateau-landon": ["7"],
  "chateau landon": ["7"],
  "poissonniere": ["7"],
  "cadet": ["7"],
  "le peletier": ["7"],
  "chaussee d'antin-la fayette": ["7", "9"],
  "chaussee dantin-la fayette": ["7", "9"],
  "pyramides": ["7", "14"],
  "pont neuf": ["7"],
  "pont marie": ["7"],
  "sully-morland": ["7"],
  "jussieu": ["7", "10"],
  "place monge": ["7"],
  "censier-daubenton": ["7"],
  "les gobelins": ["7"],
  "tolbiac": ["7"],
  "maison blanche": ["7"],
  "le kremlin-bicetre": ["7"],
  "villejuif-leo lagrange": ["7"],
  "villejuif-paul vaillant-couturier": ["7"],
  "villejuif-louis aragon": ["7"],
  "pierre et marie curie": ["7"],
  "mairie d'ivry": ["7"],
  "mairie divry": ["7"],

  // Ligne 7bis : Louis Blanc -> Pré-Saint-Gervais
  "bolivar": ["7bis"],
  "buttes chaumont": ["7bis"],
  "botzaris": ["7bis"],
  "place des fetes": ["7bis", "11"],
  "pre-saint-gervais": ["7bis"],
  "danube": ["7bis"],

  // Ligne 8 : Balard -> Pointe du Lac
  "balard": ["8"],
  "lourmel": ["8"],
  "boucicaut": ["8"],
  "felix faure": ["8"],
  "commerce": ["8"],
  "ecole militaire": ["8"],
  "la tour-maubourg": ["8"],
  "invalides": ["8", "13"],
  "madeleine": ["8", "12", "14"],
  "opera-8": ["8"],
  "richelieu-drouot": ["8", "9"],
  "grands boulevards": ["8", "9"],
  "bonne nouvelle": ["8", "9"],
  "filles du calvaire": ["8"],
  "saint-sebastien-froissart": ["8"],
  "chemin vert": ["8"],
  "ledru-rollin": ["8"],
  "faidherbe-chaligny": ["8"],
  "charonne": ["9"],
  "michel bizot": ["8"],
  "porte doree": ["8"],
  "porte de charenton": ["8"],
  "liberte": ["8"],
  "charenton-ecoles": ["8"],
  "ecole veterinaire de maisons-alfort": ["8"],
  "maisons-alfort-stade": ["8"],
  "maisons-alfort-les juilliottes": ["8"],
  "creteil-l'echat": ["8"],
  "creteil lechat": ["8"],
  "creteil-universite": ["8"],
  "creteil-prefecture": ["8"],
  "pointe du lac": ["8"],

  // Ligne 9 : Pont de Sèvres -> Mairie de Montreuil
  "pont de sevres": ["9"],
  "billancourt": ["9"],
  "marcel sembat": ["9"],
  "porte de saint-cloud": ["9"],
  "exelmans": ["9"],
  "michel-ange-molitor": ["9", "10"],
  "michel-ange-auteuil": ["9", "10"],
  "jasmin": ["9"],
  "ranelagh": ["9"],
  "la muette": ["9"],
  "rue de la pompe": ["9"],
  "iena": ["9"],
  "alma-marceau": ["9"],
  "saint-philippe du roule": ["9"],
  "miromesnil": ["9", "13"],
  "rue montmartre": ["9"],
  "voltaire": ["9"],
  "rue des boulets": ["9"],
  "nation-9": ["9"],
  "buzenval": ["9"],
  "maraichers": ["9"],
  "porte de montreuil": ["9"],
  "robespierre": ["9"],
  "croix de chavaux": ["9"],
  "mairie de montreuil": ["9"],

  // Ligne 10 : Boulogne - Pont de Saint-Cloud -> Gare d'Austerlitz
  "boulogne-pont de saint-cloud": ["10"],
  "boulogne-jean jaures": ["10"],
  "porte d'auteuil": ["10"],
  "porte dauteuil": ["10"],
  "eglise d'auteuil": ["10"],
  "eglise dauteuil": ["10"],
  "chardon-lagache": ["10"],
  "mirabeau": ["10"],
  "javel-andre citroen": ["10"],
  "charles michels": ["10"],
  "avenue emile zola": ["10"],
  "segur": ["10"],
  "duroc": ["10", "13"],
  "vaneau": ["10"],
  "sevres-babylone": ["10", "12"],
  "mabillon": ["10"],
  "cluny-la sorbonne": ["10"],
  "cardinal lemoine": ["10"],
  "maubert-mutualite": ["10"],
  "gare d'austerlitz": ["10", "5"],
  "gare dausterlitz": ["10", "5"],

  // Ligne 11 : Châtelet -> Rosny - Bois-Perrier
  "rambuteau": ["11"],
  "telegraphe": ["11"],
  "jourdain": ["11"],
  "pyrenees": ["11"],
  "goncourt": ["11"],
  "mairie des lilas": ["11"],
  "serge gainsbourg": ["11"],
  "coteaux beauclair": ["11"],
  "montreuil-hopital": ["11"],
  "la dhuys": ["11"],
  "rosny-bois-perrier": ["11"],

  // Ligne 12 : Front Populaire -> Mairie d'Issy
  "front populaire": ["12"],
  "porte de la chapelle": ["12"],
  "marx dormoy": ["12"],
  "jules joffrin": ["12"],
  "lamarck-caulaincourt": ["12"],
  "abbesses": ["12"],
  "saint-georges": ["12"],
  "notre-dame-de-lorette": ["12"],
  "trinite-d'estienne d'orves": ["12"],
  "trinite destienne dorves": ["12"],
  "assemblee nationale": ["12"],
  "solferino": ["12"],
  "rue du bac": ["12"],
  "rennes": ["12"],
  "notre-dame-des-champs": ["12"],
  "falguiere": ["12"],
  "volontaires": ["12"],
  "vaugirard": ["12"],
  "convention": ["12"],
  "porte de versailles": ["12"],
  "corentin celton": ["12"],
  "mairie d'issy": ["12"],
  "mairie dissy": ["12"],

  // Ligne 13 : Asnières-Gennevilliers / Saint-Denis - Université -> Châtillon-Montrouge
  "saint-denis-universite": ["13"],
  "basilique de saint-denis": ["13"],
  "carrefour pleyel": ["13"],
  "mairie de saint-ouen": ["13", "14"],
  "garibaldi": ["13"],
  "porte de saint-ouen": ["13"],
  "guy moquet": ["13"],
  "guy moquet-": ["13"],
  "la fourche": ["13"],
  "brochant": ["13"],
  "rome-13": ["13"],
  "liege": ["13"],
  "varenne": ["13"],
  "saint-francois-xavier": ["13"],
  "gaite": ["13"],
  "pernety": ["13"],
  "plaisance": ["13"],
  "porte de vanves": ["13"],
  "malakoff-plateau de vanves": ["13"],
  "malakoff-rue etienne dolet": ["13"],
  "chatillon-montrouge": ["13"],
  "asnieres-gennevilliers-les courtilles": ["13"],
  "les agnettes": ["13"],
  "gabriel peri": ["13"],
  "mairie de clichy": ["13"],

  // Ligne 14 : Saint-Denis - Pleyel -> Aéroport d'Orly
  "saint-denis-pleyel": ["14"],
  "porte de clichy": ["13", "14"],
  "olympiades": ["14"],
  "maison blanche-paris xiiie": ["14"],
  "hopital bicetre": ["14"],
  "villejuif-gustave roussy": ["14"],
  "chevilly-trois-communes": ["14"],
  "thiais-orly-pont de rungis": ["14"],
  "aeroport d'orly": ["14"],
  "aeroport dorly": ["14"],
  "chatelet-les halles": ["14"],
  "cour saint-emilion": ["14"],
  "bibliotheque francois mitterrand": ["14"],
};

/**
 * Cherche les lignes par nom (avec normalisation).
 * Retourne `null` si rien trouvé.
 */
export function lookupStaticLines(rawName: string): string[] | null {
  const key = normalizeStationName(rawName);
  if (key in STATION_LINES) {
    return STATION_LINES[key]!.slice().sort(compareLineRefs);
  }
  return null;
}

export function compareLineRefs(a: string, b: string): number {
  const parse = (s: string): [number, number] => {
    const m = /^(\d+)(bis)?$/.exec(s);
    if (!m) return [99, 0];
    return [Number(m[1]), m[2] === "bis" ? 1 : 0];
  };
  const [an, ab] = parse(a);
  const [bn, bb] = parse(b);
  if (an !== bn) return an - bn;
  return ab - bb;
}
