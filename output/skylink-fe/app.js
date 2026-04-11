const API_BASE = window.SKYLINK_API || '';
const WS_URL = (location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + (window.SKYLINK_API ? new URL(API_BASE).host : location.host) + '/ws';
const WS_AIS_URL = (location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + (window.SKYLINK_API ? new URL(API_BASE).host : location.host) + '/ws/ais';
let ws = null, wsAis = null, selected = null, spriteLoaded = false, vesselSpriteLoaded = false;

const map = initMap({ containerId: 'maplibreMap', center: [20, 30], zoom: 3 });

let showAircraft = true, showVessels = true;
function toggleLayer(which) {
  if (which === 'ac') {
    showAircraft = !showAircraft;
    document.getElementById('btn-ac').classList.toggle('on', showAircraft);
    ['ac-dots','ac-icons'].forEach(id => { if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', showAircraft ? 'visible' : 'none'); });
  } else {
    showVessels = !showVessels;
    document.getElementById('btn-vs').classList.toggle('on', showVessels);
    ['vessel-dots','vessel-icons'].forEach(id => { if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', showVessels ? 'visible' : 'none'); });
  }
}

function loadSprite(cb) {
  const dpr = window.devicePixelRatio >= 2 ? '@2x' : '';
  const img = new Image();
  img.crossOrigin = 'anonymous';
  img.onload = () => {
    fetch('/sprite' + dpr + '.json').then(r => r.json()).then(meta => {
      const c = document.createElement('canvas');
      c.width = img.width; c.height = img.height;
      c.getContext('2d').drawImage(img, 0, 0);
      for (const [name, m] of Object.entries(meta)) {
        const data = c.getContext('2d').getImageData(m.x, m.y, m.width, m.height);
        if (!map.hasImage(name))
          map.addImage(name, {width:m.width, height:m.height, data:data.data}, {sdf:true, pixelRatio: m.pixelRatio});
      }
      spriteLoaded = true;
      if (cb) cb();
    });
  };
  img.src = '/sprite' + dpr + '.png';
}

function loadVesselSprite(cb) {
  const dpr = window.devicePixelRatio >= 2 ? '@2x' : '';
  const img = new Image();
  img.crossOrigin = 'anonymous';
  img.onload = () => {
    fetch('/vessel-sprite' + dpr + '.json').then(r => r.json()).then(meta => {
      const c = document.createElement('canvas');
      c.width = img.width; c.height = img.height;
      c.getContext('2d').drawImage(img, 0, 0);
      for (const [name, m] of Object.entries(meta)) {
        const vname = 'v_' + name;
        const data = c.getContext('2d').getImageData(m.x, m.y, m.width, m.height);
        if (!map.hasImage(vname))
          map.addImage(vname, {width:m.width, height:m.height, data:data.data}, {sdf:true, pixelRatio: m.pixelRatio});
      }
      vesselSpriteLoaded = true;
      if (cb) cb();
    });
  };
  img.src = '/vessel-sprite' + dpr + '.png';
}

function addLayers() {
  if (map.getSource('ac')) return;

  map.addSource('ac', { type:'geojson', data:{type:'FeatureCollection',features:[]} });
  map.addSource('vessel', { type:'geojson', data:{type:'FeatureCollection',features:[]} });
  map.addSource('trace', { type:'geojson', data:{type:'FeatureCollection',features:[]} });

  map.addLayer({ id:'trace-line', type:'line', source:'trace', paint:{
    'line-color':['interpolate',['linear'],['get','alt'],
      0,'#2ecc71',15000,'#f1c40f',30000,'#e67e22',42000,'#e74c3c'],
    'line-width':2, 'line-opacity':0.7 }});

  // Vessel layer — colored ship icons by type
  map.addLayer({ id:'vessel-dots', type:'circle', source:'vessel', paint:{
    'circle-radius':['interpolate',['linear'],['zoom'], 2,2, 6,3.5, 10,5, 14,7],
    'circle-color':['match',['get','shiptype'],
      30,'#FF9800', // fishing=orange
      36,'#9C27B0', // sailing=purple
      52,'#795548', // tug=brown
      ['interpolate',['linear'],['get','shiptype'],
        0,'#9E9E9E', 59,'#9E9E9E', 60,'#2196F3', 69,'#2196F3', // passenger=blue
        70,'#4CAF50', 79,'#4CAF50', // cargo=green
        80,'#F44336', 89,'#F44336', // tanker=red
        90,'#9E9E9E']],
    'circle-opacity':0.85,
    'circle-stroke-width':1,
    'circle-stroke-color':'#000',
    'circle-stroke-opacity':0.3,
  }});

  map.addLayer({ id:'ac-dots', type:'circle', source:'ac', paint:{
    'circle-radius':['interpolate',['linear'],['zoom'], 2,1.5, 6,3, 10,5, 14,7],
    'circle-color':['interpolate',['linear'],['coalesce',['get','alt_baro'],0],
      0,'#2ecc71',5000,'#3498db',15000,'#f1c40f',30000,'#e67e22',42000,'#e74c3c'],
    'circle-opacity':0.85,
  }});

  function addIconLayer() {
    if (map.getLayer('ac-icons')) return;
    map.addLayer({ id:'ac-icons', type:'symbol', source:'ac', layout:{
      'icon-image':['coalesce',['get','_icon'],'unknown'],
      'icon-size':['interpolate',['linear'],['zoom'], 2,0.35, 5,0.5, 8,0.65, 12,0.85, 15,1.0],
      'icon-rotate':['coalesce',['get','track'],0],
      'icon-rotation-alignment':'map',
      'icon-allow-overlap':true,
      'icon-ignore-placement':true,
      'icon-padding':0,
    }, paint:{
      'icon-color':['interpolate',['linear'],['coalesce',['get','alt_baro'],0],
        0,'#2ecc71',5000,'#3498db',15000,'#f1c40f',30000,'#e67e22',42000,'#e74c3c'],
    }});
    map.setPaintProperty('ac-dots', 'circle-opacity', 0);
    map.on('click','ac-icons', clickHandler);
    map.on('mouseenter','ac-icons',()=>map.getCanvas().style.cursor='pointer');
    map.on('mouseleave','ac-icons',()=>map.getCanvas().style.cursor='');
  }

  if (spriteLoaded) addIconLayer();
  else loadSprite(addIconLayer);

  function addVesselIconLayer() {
    if (map.getLayer('vessel-icons')) return;
    map.addLayer({ id:'vessel-icons', type:'symbol', source:'vessel', layout:{
      'icon-image':['coalesce',['get','_vicon'],'v_v_default'],
      'icon-size':['interpolate',['linear'],['zoom'], 2,0.4, 5,0.55, 8,0.7, 12,0.9, 15,1.1],
      'icon-rotate':['coalesce',['get','heading'],['get','cog'],0],
      'icon-rotation-alignment':'map',
      'icon-allow-overlap':true,
      'icon-ignore-placement':true,
      'icon-padding':0,
    }, paint:{
      'icon-color':['match',['get','shiptype'],
        30,'#FF9800', 36,'#9C27B0', 52,'#795548',
        ['interpolate',['linear'],['get','shiptype'],
          0,'#9E9E9E', 59,'#9E9E9E', 60,'#2196F3', 69,'#2196F3',
          70,'#4CAF50', 79,'#4CAF50', 80,'#F44336', 89,'#F44336', 90,'#9E9E9E']],
    }});
  }

  if (vesselSpriteLoaded) addVesselIconLayer();
  else loadVesselSprite(addVesselIconLayer);

  const clickHandler = e => {
    const p = e.features[0].properties;
    selected = p.hex;
    document.getElementById('ph').textContent = p.flight || p.hex;
    document.getElementById('pb').innerHTML = [
      ['ICAO',p.hex],['Reg',p.r||'—'],['Type',p.t||'—'],
      ['Alt',p.alt_baro?p.alt_baro+' ft':'—'],
      ['GS',p.gs?Math.round(p.gs)+' kt':'—'],['Trk',p.track?Math.round(p.track)+'°':'—'],
      ['Sqk',p.squawk||'—'],['Cat',p.category||'—'],
    ].map(([l,v])=>`<div class="r"><span class="l">${l}</span><span>${v}</span></div>`).join('');
    document.getElementById('panel').style.display='block';
    loadTrace(p.hex);
  };
  map.on('click','ac-dots', clickHandler);
  map.on('mouseenter','ac-dots',()=>map.getCanvas().style.cursor='pointer');
  map.on('mouseleave','ac-dots',()=>map.getCanvas().style.cursor='');

  // Vessel click
  const vesselClickHandler = e => {
    const p = e.features[0].properties;
    selected = 'v:' + p.mmsi;
    document.getElementById('ph').textContent = p.shipname || 'MMSI ' + p.mmsi;
    document.getElementById('pb').innerHTML = [
      ['MMSI',p.mmsi],['Name',p.shipname||'—'],['Call',p.callsign||'—'],
      ['Type',p.shiptype + ' (' + (resolveVesselIcon(p)) + ')'],
      ['Speed',p.speed!=null?p.speed.toFixed(1)+' kt':'—'],
      ['COG',p.cog!=null?p.cog.toFixed(1)+'°':'—'],
      ['Hdg',p.heading!=null?p.heading+'°':'—'],
      ['IMO',p.imo||'—'],['Status',p.status!=null?p.status:'—'],
    ].map(([l,v])=>`<div class="r"><span class="l">${l}</span><span>${v}</span></div>`).join('');
    document.getElementById('panel').style.display='block';
  };
  map.on('click','vessel-dots', vesselClickHandler);
  map.on('mouseenter','vessel-dots',()=>map.getCanvas().style.cursor='pointer');
  map.on('mouseleave','vessel-dots',()=>map.getCanvas().style.cursor='');
  map.on('click','vessel-icons', vesselClickHandler);
  map.on('mouseenter','vessel-icons',()=>map.getCanvas().style.cursor='pointer');
  map.on('mouseleave','vessel-icons',()=>map.getCanvas().style.cursor='');

  map.on('click', e => {
    const layers = map.getLayer('ac-icons') ? ['ac-icons','ac-dots','vessel-dots','vessel-icons'] : ['ac-dots','vessel-dots'];
    if (!map.queryRenderedFeatures(e.point,{layers}).length) closePanel();
  });
}

function closePanel() {
  document.getElementById('panel').style.display='none';
  selected = null;
  if (map.getSource('trace')) map.getSource('trace').setData({type:'FeatureCollection',features:[]});
}

async function loadTrace(hex) {
  try {
    const r = await fetch(API_BASE+'/data/traces/'+hex+'/trace_recent.json');
    if (!r.ok) return;
    const t = await r.json();
    if (!t.trace || !t.trace.length) return;
    const segments = [];
    for (let i = 1; i < t.trace.length; i++) {
      const p = t.trace[i-1], c = t.trace[i];
      if (p[1] && p[2] && c[1] && c[2])
        segments.push({type:'Feature',properties:{alt:c[3]||0},
          geometry:{type:'LineString',coordinates:[[p[2],p[1]],[c[2],c[1]]]}});
    }
    if (map.getSource('trace'))
      map.getSource('trace').setData({type:'FeatureCollection',features:segments});
  } catch(e) {}
}

function connectWS() {
  ws = new WebSocket(WS_URL);
  ws.binaryType = 'arraybuffer';
  ws.onopen = () => sendBbox();
  ws.onmessage = e => {
    try {
      let data;
      if (e.data instanceof ArrayBuffer) {
        data = decodeBinCraft(e.data);
      } else {
        data = JSON.parse(e.data);
      }
      for (const f of data.features) f.properties._icon = resolveIcon(f.properties);
      if (map.getSource('ac')) map.getSource('ac').setData(data);
      document.getElementById('hud').textContent = data.features.length + ' ac';
    } catch(err) { console.error('ws decode:', err); }
  };
  ws.onclose = () => setTimeout(connectWS, 2000);
  ws.onerror = () => ws.close();
}

function sendBbox() {
  if (!ws || ws.readyState !== 1) return;
  const b = map.getBounds(), z = map.getZoom(), p = 3/(z+1);
  const box = 'box:'+[
    Math.max(-90,b.getSouth()-p).toFixed(1),
    Math.min(90,b.getNorth()+p).toFixed(1),
    Math.max(-180,b.getWest()-p).toFixed(1),
    Math.min(180,b.getEast()+p).toFixed(1)].join(',');
  ws.send(box);
  if (wsAis && wsAis.readyState === 1) wsAis.send(box);
}

function connectVesselWS() {
  wsAis = new WebSocket(WS_AIS_URL);
  wsAis.binaryType = 'arraybuffer';
  wsAis.onopen = () => sendBbox();
  wsAis.onmessage = e => {
    try {
      const data = (e.data instanceof ArrayBuffer) ? decodeBinVessel(e.data) : JSON.parse(e.data);
      for (const f of data.features) f.properties._vicon = 'v_' + resolveVesselIcon(f.properties);
      if (map.getSource('vessel')) map.getSource('vessel').setData(data);
      document.getElementById('hud').textContent =
        (map.getSource('ac') ? (map.getSource('ac')._data?.features?.length || 0) + ' ac  ' : '') +
        data.features.length + ' vessels';
    } catch(err) { console.error('ws/ais decode:', err); }
  };
  wsAis.onclose = () => setTimeout(connectVesselWS, 2000);
  wsAis.onerror = () => wsAis.close();
}

map.on('style.load', () => { spriteLoaded = false; vesselSpriteLoaded = false; addLayers(); });
map.on('load', () => { addLayers(); connectWS(); connectVesselWS(); map.on('moveend', sendBbox); });
