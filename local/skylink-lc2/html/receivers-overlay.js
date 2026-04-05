// Receiver Overlay for tar1090 - Step 1: plot dots (proven working pattern)
(function() {
'use strict';

var src = new ol.source.Vector();
var covSrc = new ol.source.Vector();
var data = [];
var refreshTimer = null;

var dotStyle = new ol.style.Style({
  image: new ol.style.Circle({ radius: 5, fill: new ol.style.Fill({ color: 'rgba(0,212,255,0.8)' }) })
});
var dotStyleGood = new ol.style.Style({
  image: new ol.style.Circle({ radius: 6, fill: new ol.style.Fill({ color: 'rgba(46,213,115,0.9)' }) })
});
var dotStyleWarn = new ol.style.Style({
  image: new ol.style.Circle({ radius: 4, fill: new ol.style.Fill({ color: 'rgba(255,165,2,0.8)' }) })
});
var dotStyleBad = new ol.style.Style({
  image: new ol.style.Circle({ radius: 3, fill: new ol.style.Fill({ color: 'rgba(255,71,87,0.7)' }) })
});
var covStyle = new ol.style.Style({
  stroke: new ol.style.Stroke({ color: 'rgba(0,212,255,0.25)', width: 1, lineDash: [4,4] }),
  fill: new ol.style.Fill({ color: 'rgba(0,212,255,0.03)' })
});

function pickStyle(rate) {
  if (rate > 10) return dotStyleGood;
  if (rate > 3) return dotStyleWarn;
  if (rate > 0.5) return dotStyleBad;
  return dotStyle;
}

// --- Geo cache ---
var GEO_KEY = 'rcv_geo_v2';
var geoCache = {};
try { geoCache = JSON.parse(localStorage.getItem(GEO_KEY)) || {}; } catch(e) { geoCache = {}; }
function saveGeo() { try { localStorage.setItem(GEO_KEY, JSON.stringify(geoCache)); } catch(e) {} }

var flagMap = {"Afghanistan":"\u{1F1E6}\u{1F1EB}","Argentina":"\u{1F1E6}\u{1F1F7}","Australia":"\u{1F1E6}\u{1F1FA}","Austria":"\u{1F1E6}\u{1F1F9}","Belgium":"\u{1F1E7}\u{1F1EA}","Brazil":"\u{1F1E7}\u{1F1F7}","Bulgaria":"\u{1F1E7}\u{1F1EC}","Canada":"\u{1F1E8}\u{1F1E6}","Chile":"\u{1F1E8}\u{1F1F1}","China":"\u{1F1E8}\u{1F1F3}","Colombia":"\u{1F1E8}\u{1F1F4}","Croatia":"\u{1F1ED}\u{1F1F7}","Czechia":"\u{1F1E8}\u{1F1FF}","Czech Republic":"\u{1F1E8}\u{1F1FF}","Denmark":"\u{1F1E9}\u{1F1F0}","Egypt":"\u{1F1EA}\u{1F1EC}","Estonia":"\u{1F1EA}\u{1F1EA}","Finland":"\u{1F1EB}\u{1F1EE}","France":"\u{1F1EB}\u{1F1F7}","Germany":"\u{1F1E9}\u{1F1EA}","Greece":"\u{1F1EC}\u{1F1F7}","Hong Kong":"\u{1F1ED}\u{1F1F0}","Hungary":"\u{1F1ED}\u{1F1FA}","Iceland":"\u{1F1EE}\u{1F1F8}","India":"\u{1F1EE}\u{1F1F3}","Indonesia":"\u{1F1EE}\u{1F1E9}","Ireland":"\u{1F1EE}\u{1F1EA}","Israel":"\u{1F1EE}\u{1F1F1}","Italy":"\u{1F1EE}\u{1F1F9}","Japan":"\u{1F1EF}\u{1F1F5}","Latvia":"\u{1F1F1}\u{1F1FB}","Lithuania":"\u{1F1F1}\u{1F1F9}","Luxembourg":"\u{1F1F1}\u{1F1FA}","Malaysia":"\u{1F1F2}\u{1F1FE}","Malta":"\u{1F1F2}\u{1F1F9}","Mexico":"\u{1F1F2}\u{1F1FD}","Netherlands":"\u{1F1F3}\u{1F1F1}","New Zealand":"\u{1F1F3}\u{1F1FF}","Nigeria":"\u{1F1F3}\u{1F1EC}","Norway":"\u{1F1F3}\u{1F1F4}","Pakistan":"\u{1F1F5}\u{1F1F0}","Peru":"\u{1F1F5}\u{1F1EA}","Philippines":"\u{1F1F5}\u{1F1ED}","Poland":"\u{1F1F5}\u{1F1F1}","Portugal":"\u{1F1F5}\u{1F1F9}","Romania":"\u{1F1F7}\u{1F1F4}","Russia":"\u{1F1F7}\u{1F1FA}","Saudi Arabia":"\u{1F1F8}\u{1F1E6}","Serbia":"\u{1F1F7}\u{1F1F8}","Singapore":"\u{1F1F8}\u{1F1EC}","Slovakia":"\u{1F1F8}\u{1F1F0}","Slovenia":"\u{1F1F8}\u{1F1EE}","South Africa":"\u{1F1FF}\u{1F1E6}","South Korea":"\u{1F1F0}\u{1F1F7}","Spain":"\u{1F1EA}\u{1F1F8}","Sweden":"\u{1F1F8}\u{1F1EA}","Switzerland":"\u{1F1E8}\u{1F1ED}","Taiwan":"\u{1F1F9}\u{1F1FC}","Thailand":"\u{1F1F9}\u{1F1ED}","Turkey":"\u{1F1F9}\u{1F1F7}","T\u00fcrkiye":"\u{1F1F9}\u{1F1F7}","Ukraine":"\u{1F1FA}\u{1F1E6}","United Arab Emirates":"\u{1F1E6}\u{1F1EA}","United Kingdom":"\u{1F1EC}\u{1F1E7}","United States":"\u{1F1FA}\u{1F1F8}","United States of America":"\u{1F1FA}\u{1F1F8}","Vietnam":"\u{1F1FB}\u{1F1F3}","Viet Nam":"\u{1F1FB}\u{1F1F3}"};
function getFlag(c) { return flagMap[c] || '\u{1F30D}'; }

var lookupQueue = [], lookupBusy = false;
function processLookup() {
  if (lookupBusy || !lookupQueue.length) return;
  lookupBusy = true;
  var item = lookupQueue.shift();
  fetch('https://photon.komoot.io/reverse?lon=' + item.lon.toFixed(4) + '&lat=' + item.lat.toFixed(4))
    .then(function(r) { return r.json(); })
    .then(function(d) {
      var f = d.features && d.features[0], p = f ? f.properties : {};
      geoCache[item.uuid] = { country: p.country || 'Unknown', city: p.city || p.state || '', flag: getFlag(p.country || '') };
      saveGeo();
    })
    .catch(function() { geoCache[item.uuid] = { country: 'Unknown', city: '', flag: '\u{1F30D}' }; })
    .finally(function() { lookupBusy = false; setTimeout(processLookup, 200); });
}

function getGeo(uuid, lat, lon) {
  if (geoCache[uuid]) return geoCache[uuid];
  if (!lookupQueue.some(function(q) { return q.uuid === uuid; }))
    lookupQueue.push({ uuid: uuid, lat: lat, lon: lon });
  if (!lookupBusy) setTimeout(processLookup, 10);
  return null;
}

// --- Layers ---
var lyr, covLyr, popup, popupEl;

function init() {
  if (typeof OLMap === 'undefined' || !OLMap) { setTimeout(init, 500); return; }

  lyr = new ol.layer.Vector({
    name: 'receivers', type: 'overlay', title: '\u{1F4E1} Receivers',
    source: src, zIndex: 45, visible: false
  });
  covLyr = new ol.layer.Vector({
    name: 'receiver_coverage', type: 'overlay', title: '\u{1F4E1} Receiver coverage',
    source: covSrc, zIndex: 44, visible: false, minZoom: 3.5, style: covStyle
  });

  OLMap.addLayer(covLyr);
  OLMap.addLayer(lyr);
  OLMap.getControls().forEach(function(c) { if (c.renderPanel) c.renderPanel(); });

  // Popup
  popupEl = document.createElement('div');
  popupEl.style.cssText = 'background:#111827ee;color:#c8d6e5;padding:8px 12px;border-radius:6px;border:1px solid #1e3a5f;font-size:12px;line-height:1.5;pointer-events:none;white-space:nowrap;box-shadow:0 4px 12px rgba(0,0,0,.5)';
  popup = new ol.Overlay({ element: popupEl, positioning: 'bottom-center', offset: [0, -10], stopEvent: false });
  OLMap.addOverlay(popup);

  OLMap.on('pointermove', function(evt) {
    if (!lyr.getVisible()) { popup.setPosition(undefined); return; }
    var feature = OLMap.forEachFeatureAtPixel(evt.pixel, function(f) { return f.get('rcv_uuid') ? f : null; });
    if (feature) {
      var r = feature.get('rcv_data');
      var geo = getGeo(r.uuid, r.lat, r.lon);
      var geoStr = geo ? ('<b>' + geo.flag + ' ' + geo.country + '</b>' + (geo.city ? ' ' + geo.city : '')) : '<i>loading...</i>';
      popupEl.innerHTML = geoStr + '<br>' +
        '<span style="color:#00d4ff;font-family:monospace;font-size:11px">' + r.uuid + '</span><br>' +
        'Rate: <b style="color:#2ed573">' + r.rate.toFixed(1) + '</b> pos/s \u00b7 T/O: ' + r.timeouts.toFixed(1) + '/hr<br>' +
        'Coverage: ' + (r.latMax - r.latMin).toFixed(1) + '\u00b0\u00d7' + (r.lonMax - r.lonMin).toFixed(1) + '\u00b0' +
        (r.bad ? '<br><span style="color:#ff4757">\u26a0 Bad Extent</span>' : '');
      popup.setPosition(evt.coordinate);
    } else { popup.setPosition(undefined); }
  });

  lyr.on('change:visible', function() {
    if (lyr.getVisible()) {
      doFetch();
      if (!refreshTimer) refreshTimer = setInterval(doFetch, 30000);
    } else {
      clearInterval(refreshTimer); refreshTimer = null;
      popup.setPosition(undefined);
      src.clear(); covSrc.clear(); data = [];
    }
  });

  console.log('receivers-overlay: ready, geo cache: ' + Object.keys(geoCache).length);
}

function doFetch() {
  fetch('data/receivers.json').then(function(r) { return r.json(); }).then(function(d) {
    var recs = d.receivers || [];
    src.clear();
    covSrc.clear();
    data = [];
    for (var i = 0; i < recs.length; i++) {
      var r = recs[i];
      var item = { uuid: r[0], rate: r[1], timeouts: r[2], latMin: r[3], latMax: r[4], lonMin: r[5], lonMax: r[6], bad: r[7], lat: r[8], lon: r[9] };
      data.push(item);

      var f = new ol.Feature(new ol.geom.Point(ol.proj.fromLonLat([item.lon, item.lat])));
      f.setStyle(pickStyle(item.rate));
      f.set('rcv_uuid', item.uuid);
      f.set('rcv_data', item);
      src.addFeature(f);

      if ((item.latMax - item.latMin) > 0.05) {
        var box = new ol.Feature(ol.geom.Polygon.fromExtent(
          ol.proj.transformExtent([item.lonMin, item.latMin, item.lonMax, item.latMax], 'EPSG:4326', 'EPSG:3857')
        ));
        box.set('rcv_uuid', item.uuid);
        box.set('rcv_data', item);
        covSrc.addFeature(box);
      }
    }
    console.log('receivers-overlay: plotted ' + data.length + ' receivers');
  }).catch(function(e) { console.error('receivers-overlay:', e); });
}

init();
})();
