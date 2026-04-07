// Vessel icon resolver: shiptype → icon name
// Uses the aircraft sprite for now (reuses shapes), vessel-specific sprite later

const VesselTypeIcons = {
  // Fishing
  30: 'fishing',
  // Towing
  31: 'tug', 32: 'tug',
  // Dredging/Diving/Military
  33: 'cargo', 34: 'cargo', 35: 'military',
  // Sailing/Pleasure
  36: 'sailing', 37: 'yacht',
  // HSC
  40: 'hsc', 41: 'hsc', 42: 'hsc', 43: 'hsc', 44: 'hsc', 45: 'hsc', 46: 'hsc', 47: 'hsc', 48: 'hsc', 49: 'hsc',
  // Pilot/SAR/Tug/Port
  50: 'pilot', 51: 'sar', 52: 'tug', 53: 'tender', 54: 'cargo', 55: 'law',
  58: 'medical',
  // Passenger
  60: 'passenger', 61: 'passenger', 62: 'passenger', 63: 'passenger', 64: 'passenger',
  65: 'passenger', 66: 'passenger', 67: 'passenger', 68: 'passenger', 69: 'passenger',
  // Cargo
  70: 'cargo', 71: 'cargo_a', 72: 'cargo_b', 73: 'cargo_c', 74: 'cargo_d',
  75: 'cargo', 76: 'cargo', 77: 'cargo', 78: 'cargo', 79: 'cargo',
  // Tanker
  80: 'tanker', 81: 'tanker_a', 82: 'tanker_b', 83: 'tanker_c', 84: 'tanker_d',
  85: 'tanker', 86: 'tanker', 87: 'tanker', 88: 'tanker', 89: 'tanker',
};

const VesselClassIcons = {
  3: 'basestation',
  4: 'sar_aircraft',
  5: 'aton',
};

// Simple SVG vessel shapes as data URIs for MapLibre
const VESSEL_SHAPES = {
  cargo:      { path: 'M4 2L12 0L20 2L20 22L12 24L4 22Z', color: '#4CAF50' },
  tanker:     { path: 'M4 2L12 0L20 2L20 22L12 24L4 22Z', color: '#F44336' },
  passenger:  { path: 'M4 2L12 0L20 2L20 22L12 24L4 22Z', color: '#2196F3' },
  fishing:    { path: 'M4 4L12 0L20 4L18 22L6 22Z', color: '#FF9800' },
  sailing:    { path: 'M12 0L20 8L16 24L8 24L4 8Z', color: '#9C27B0' },
  tug:        { path: 'M6 2L18 2L20 12L18 20L6 20L4 12Z', color: '#795548' },
  hsc:        { path: 'M4 4L12 0L20 4L18 22L6 22Z', color: '#00BCD4' },
  sar:        { path: 'M12 0L22 10L12 24L2 10Z', color: '#FF5722' },
  pilot:      { path: 'M8 2L16 2L18 20L6 20Z', color: '#FFC107' },
  aton:       { path: 'M12 2L20 12L12 22L4 12Z', color: '#E91E63' },
  default:    { path: 'M6 2L18 2L20 12L18 22L6 22L4 12Z', color: '#9E9E9E' },
};

function resolveVesselIcon(props) {
  // Check class-specific icons first
  if (VesselClassIcons[props.shipclass]) return VesselClassIcons[props.shipclass];
  // Then type-specific
  if (VesselTypeIcons[props.shiptype]) return VesselTypeIcons[props.shiptype];
  // Fallback by type range
  const t = props.shiptype || 0;
  if (t >= 60 && t <= 69) return 'passenger';
  if (t >= 70 && t <= 79) return 'cargo';
  if (t >= 80 && t <= 89) return 'tanker';
  if (t >= 40 && t <= 49) return 'hsc';
  if (t === 30) return 'fishing';
  if (t === 36) return 'sailing';
  return 'default';
}
