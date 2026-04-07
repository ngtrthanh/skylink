// binVessel zstd decoder → GeoJSON FeatureCollection
// Decodes binary vessel frames from WS into MapLibre-compatible GeoJSON
// Stride: 64 bytes per vessel

function decodeBinVessel(compressed) {
  const raw = fzstd.decompress(new Uint8Array(compressed));
  const buf = raw.buffer;
  const stride = new Uint32Array(buf, 8, 1)[0]; // offset 8: stride
  const features = [];

  for (let off = stride; off + stride <= buf.byteLength; off += stride) {
    const u32 = new Uint32Array(buf, off, stride / 4);
    const s32 = new Int32Array(buf, off, stride / 4);
    const u16 = new Uint16Array(buf, off, stride / 2);
    const s16 = new Int16Array(buf, off, stride / 2);
    const u8  = new Uint8Array(buf, off, stride);

    const vb = u8[21]; // validity bits
    if (!(vb & 1)) continue; // no position

    const lat = s32[1] / 1e6; // offset 4
    const lon = s32[2] / 1e6; // offset 8
    if (lat === 0 && lon === 0) continue;

    const mmsi = u32[0];
    const p = { mmsi };

    if (vb & 2)   p.speed = u16[6] / 10;    // offset 12
    if (vb & 4)   p.cog = u16[7] / 10;      // offset 14
    if (vb & 8)   p.heading = u16[8];        // offset 16
    if (vb & 16)  p.status = u8[18];
    p.shiptype = u8[19];
    p.shipclass = u8[20];

    if (vb & 64)  p.imo = u32[6];           // offset 24

    // shipname (20 bytes at offset 28)
    if (vb & 32) {
      let name = '';
      for (let i = 28; u8[i] && i < 48; i++) name += String.fromCharCode(u8[i]);
      p.shipname = name;
    }

    // callsign (7 bytes at offset 48)
    if (vb & 128) {
      let cs = '';
      for (let i = 48; u8[i] && i < 55; i++) cs += String.fromCharCode(u8[i]);
      p.callsign = cs;
    }

    features.push({
      type: 'Feature',
      geometry: { type: 'Point', coordinates: [lon, lat] },
      properties: p,
    });
  }

  return { type: 'FeatureCollection', features };
}
