// binCraft zstd decoder → GeoJSON FeatureCollection
// Decodes binary frames from WS into MapLibre-compatible GeoJSON

function decodeBinCraft(compressed) {
  const raw = fzstd.decompress(new Uint8Array(compressed));
  const buf = raw.buffer;
  const hdr = new Uint32Array(buf, 0, 13);
  const stride = hdr[2];
  const features = [];

  for (let off = stride; off < buf.byteLength; off += stride) {
    const u32 = new Uint32Array(buf, off, stride / 4);
    const s32 = new Int32Array(buf, off, stride / 4);
    const u16 = new Uint16Array(buf, off, stride / 2);
    const s16 = new Int16Array(buf, off, stride / 2);
    const u8  = new Uint8Array(buf, off, stride);

    // validity bits
    const v0 = u8[73], v1 = u8[74], v2 = u8[75], v3 = u8[76];

    // position required
    if (!(v0 & 64)) continue;

    const lat = s32[3] / 1e6;
    const lon = s32[2] / 1e6;
    if (lat === 0 && lon === 0) continue;

    const hex = (s32[0] & 0xFFFFFF).toString(16).padStart(6, '0');

    // strings
    let flight = '';
    for (let i = 78; u8[i] && i < 86; i++) flight += String.fromCharCode(u8[i]);
    let t = '';
    for (let i = 88; u8[i] && i < 92; i++) t += String.fromCharCode(u8[i]);
    let r = '';
    for (let i = 92; u8[i] && i < 104; i++) r += String.fromCharCode(u8[i]);

    const p = { hex };
    if (v0 & 8)   p.flight = flight;
    if (v0 & 16)  p.alt_baro = s16[10] * 25;
    if (v0 & 128) p.gs = s16[17] / 10;
    if (v1 & 8)   p.track = s16[20] / 90;
    if (v3 & 4) {
      const sq = u16[16].toString(16).padStart(4, '0');
      p.squawk = sq[0] > '9' ? String(parseInt(sq[0],16)) + sq.slice(1) : sq;
    }
    if (u8[64])   p.category = u8[64].toString(16).toUpperCase();
    if (v1 & 1)   p.ias = u16[29];
    if (v1 & 4)   p.mach = s16[18] / 1000;
    if (v0 & 32)  p.alt_geom = s16[11] * 25;
    if (v2 & 1)   p.baro_rate = s16[8] * 8;
    if (t)        p.t = t;
    if (r)        p.r = r;

    // type description + wtc from db (encoded in bincraft by our backend)
    // Our backend doesn't encode desc/wtc in bincraft — we'll resolve from t via iconMap

    features.push({
      type: 'Feature',
      geometry: { type: 'Point', coordinates: [lon, lat] },
      properties: p,
    });
  }

  return { type: 'FeatureCollection', features };
}
