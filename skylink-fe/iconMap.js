// Aircraft icon mapping — maps category/typeDesignator/typeDescription to sprite shape names
// 81 shapes available in sprite: airliner, cessna, jet_swept, helicopter, heavy_2e, heavy_4e,
// hi_perf, glider, balloon, uav, para, f18, f35, typhoon, rafale, tornado, apache, blackhawk,
// chinook, c130, c17, c5, a380, a225, b52, b707, md11, beluga, etc.

const TypeDesignatorIcons = {
  A10:'a10',A148:'hi_perf',A3:'hi_perf',A318:'airliner',A319:'airliner',A320:'airliner',A321:'airliner',
  A37:'hi_perf',A388:'a380',A6:'hi_perf',A700:'hi_perf',AJET:'alpha_jet',AT3:'hi_perf',
  B712:'jet_swept',B731:'airliner',B732:'airliner',B733:'airliner',B734:'airliner',B735:'airliner',
  B736:'airliner',B737:'airliner',B738:'airliner',B739:'airliner',B752:'airliner',B762:'airliner',
  B741:'heavy_4e',B742:'heavy_4e',B743:'heavy_4e',B744:'heavy_4e',B748:'heavy_4e',B74D:'heavy_4e',
  B74R:'heavy_4e',B74S:'heavy_4e',B772:'heavy_2e',B773:'heavy_2e',B77L:'heavy_2e',B77W:'heavy_2e',
  BLCF:'heavy_2e',BSCA:'heavy_4e',
  C650:'jet_swept',C750:'jet_swept',CKUO:'hi_perf',CL30:'jet_swept',CL35:'jet_swept',CL60:'jet_swept',
  CRJ1:'jet_swept',CRJ2:'jet_swept',CRJ7:'jet_swept',CRJ9:'jet_swept',
  DC10:'md11',DH8A:'twin_small',DH8B:'twin_small',DH8C:'twin_small',DH8D:'twin_small',
  E135:'jet_swept',E145:'jet_swept',E170:'jet_swept',E45X:'jet_swept',
  EMER:'ground_emergency',EUFI:'typhoon',
  F1:'hi_perf',F100:'hi_perf',F111:'hi_perf',F117:'hi_perf',F14:'hi_perf',F15:'md_f15',
  F16:'hi_perf',F18:'f18',F22:'hi_perf',F22A:'hi_perf',F35:'f35',F4:'hi_perf',F5:'f5_tiger',
  FOUG:'hi_perf',
  GL5T:'jet_swept',GLF2:'jet_swept',GLF3:'jet_swept',GLF4:'jet_swept',GLF5:'jet_swept',GLF6:'jet_swept',
  GND:'ground_unknown',
  H25A:'jet_swept',H25B:'jet_swept',H25C:'jet_swept',
  J8A:'hi_perf',J8B:'hi_perf',JH7:'hi_perf',
  LEOP:'hi_perf',LTNG:'hi_perf',
  MD11:'md11',MD80:'jet_swept',MD81:'jet_swept',MD82:'jet_swept',MD83:'jet_swept',MD87:'jet_swept',MD88:'jet_swept',
  ME62:'hi_perf',METR:'hi_perf',MG19:'hi_perf',MG25:'hi_perf',MG29:'hi_perf',MG31:'hi_perf',MG44:'hi_perf',
  MIR4:'mirage',MT2:'hi_perf',
  Q5:'hi_perf',RFAL:'rafale',
  S3:'hi_perf',S37:'hi_perf',SERV:'ground_service',SR71:'hi_perf',
  SU15:'hi_perf',SU24:'hi_perf',SU25:'hi_perf',SU27:'hi_perf',
  T2:'hi_perf',T22M:'hi_perf',T37:'hi_perf',T38:'t38',T4:'hi_perf',TOR:'tornado',
  TU22:'hi_perf',TWR:'ground_tower',
  VAUT:'hi_perf',WB57:'wb57',Y130:'hi_perf',YK28:'hi_perf',
  // Additional specific types
  A124:'a225',A225:'a225',AN22:'heavy_4e',AN12:'c130',
  A332:'heavy_2e',A333:'heavy_2e',A339:'heavy_2e',A340:'heavy_4e',A342:'heavy_4e',A343:'heavy_4e',A345:'heavy_4e',A346:'heavy_4e',
  A359:'heavy_2e',A35K:'heavy_2e',
  B1:'b1b_lancer',B2:'strato',B52:'b52',
  C130:'c130',C17:'c17',C5:'c5',C5M:'c5',
  E3CF:'e3awacs',E3TF:'e3awacs',E6:'e3awacs',
  HAWK:'bae_hawk',
  IL62:'il_62',IL76:'c130',IL96:'heavy_4e',
  L159:'l159',
  P3:'p3_orion',P8:'p3_orion',
  TUCA:'typhoon',TYPH:'typhoon',EF2K:'typhoon',
  GRIF:'rafale',
  V22:'v22_slow',
  U2:'u2',
  A10T:'a10',A10A:'a10',
  AH64:'apache',AH1:'helicopter',
  UH60:'blackhawk',S70:'blackhawk',
  CH47:'chinook',CH53:'s61',
  AS32:'puma',AS33:'puma',EC25:'puma',
  EC35:'helicopter',EC45:'helicopter',EC55:'helicopter',EC65:'dauphin',
  AS65:'dauphin',SA36:'gazelle',
  R22:'helicopter',R44:'helicopter',R66:'helicopter',
  B06:'helicopter',B07:'helicopter',B12:'helicopter',B47G:'helicopter',
  MI24:'mil24',MI28:'mil24',MI8:'mil24',MI17:'mil24',
  AS50:'helicopter',AS55:'helicopter',
  EC30:'helicopter',EC20:'helicopter',
  BALL:'balloon',
  GLID:'glider',ASK2:'glider',DG80:'glider',
  ULAC:'cessna',
  SR22:'cirrus_sr22',SR20:'cirrus_sr22',
  C172:'cessna',C152:'cessna',C182:'cessna',C206:'cessna',C210:'cessna',PA28:'pa24',PA32:'pa24',PA34:'twin_small',
  BE36:'cessna',BE58:'twin_small',BE9L:'twin_small',BE20:'twin_small',
  PC12:'single_turbo',TBM7:'single_turbo',TBM8:'single_turbo',TBM9:'single_turbo',
  AT72:'twin_large',AT75:'twin_large',AT76:'twin_large',
  SF34:'twin_small',SF50:'jet_swept',
  BELF:'beluga',
  A3ST:'super_guppy',
  LANC:'lancaster',
  HUNT:'hunter',
  PUMA:'puma',TIGR:'tiger',
  RQ4:'uav',MQ1:'uav',MQ9:'uav',
};

const TypeDescriptionIcons = {
  H1P:'helicopter',H1T:'helicopter',H2P:'helicopter',H2T:'helicopter',H3T:'helicopter',
  L1J:'hi_perf',L1P:'cessna',L1T:'single_turbo',
  'L2J-H':'heavy_2e','L2J-L':'jet_swept','L2J-M':'airliner',
  L2P:'twin_large',L2T:'twin_small',
  L4J:'heavy_4e',L4T:'heavy_4e',
};

const CategoryIcons = {
  A0:'unknown',A1:'cessna',A2:'jet_nonswept',A3:'airliner',A4:'heavy_2e',A5:'heavy_4e',
  A6:'hi_perf',A7:'helicopter',
  B1:'glider',B2:'balloon',B4:'para',B6:'uav',
  C0:'ground_unknown',C1:'ground_emergency',C2:'ground_service',C3:'ground_tower',
};

// Resolve icon name from aircraft properties
// Priority: typeDesignator → typeDescription+wtc → typeDescription → category → 'unknown'
function resolveIcon(props) {
  if (props.t && TypeDesignatorIcons[props.t]) return TypeDesignatorIcons[props.t];
  if (props.desc) {
    const key = props.wtc ? props.desc + '-' + props.wtc : null;
    if (key && TypeDescriptionIcons[key]) return TypeDescriptionIcons[key];
    if (TypeDescriptionIcons[props.desc]) return TypeDescriptionIcons[props.desc];
  }
  if (props.category && CategoryIcons[props.category]) return CategoryIcons[props.category];
  return 'unknown';
}
