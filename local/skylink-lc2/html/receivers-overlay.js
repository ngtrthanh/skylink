(function() {
// Load flag-icons CSS
var link = document.createElement('link');
link.rel = 'stylesheet';
link.href = 'https://cdn.jsdelivr.net/gh/lipis/flag-icons@7.2.3/css/flag-icons.min.css';
document.head.appendChild(link);

var srcG = new ol.source.Vector();
var srcO = new ol.source.Vector();
var srcR = new ol.source.Vector();
var srcX = new ol.source.Vector();
var allData = [];
var refreshTimer = null;
var layers = [];

// --- Geo cache: {uuid: {cc:"us", country:"United States", city:"New York"}} ---
var GEO_KEY = 'rcv_geo_v7';
var geoCache = {};
try { geoCache = JSON.parse(localStorage.getItem(GEO_KEY)) || {}; } catch(e) { geoCache = {}; }
function saveGeo() { try { localStorage.setItem(GEO_KEY, JSON.stringify(geoCache)); } catch(e) {} }

// Bbox fallback for instant display
var CC = [
  // Small Asian countries BEFORE China
  ['vn','Vietnam',8,24,102,110],['tw','Taiwan',21,26,119,122],
  ['th','Thailand',5,21,97,106],['kh','Cambodia',10,15,102,108],
  ['la','Laos',14,23,100,108],['mm','Myanmar',10,28,92,101],
  ['sg','Singapore',1,2,103,104],['my','Malaysia',0,8,99,120],
  ['ph','Philippines',5,21,117,127],['id','Indonesia',-11,6,95,141],
  ['kr','South Korea',33,39,125,130],['jp','Japan',30,46,129,146],
  ['np','Nepal',26,31,80,89],['bd','Bangladesh',20,27,88,93],
  ['lk','Sri Lanka',6,10,79,82],
  // Then large Asian countries
  ['in','India',6,36,68,98],['cn','China',18,54,73,135],
  // Small European countries BEFORE Russia
  ['gb','UK',49,61,-8,2],['ie','Ireland',51,56,-11,-5],
  ['de','Germany',47,55,6,15],['fr','France',42,51,-5,9],
  ['nl','Netherlands',50,54,3,8],['be','Belgium',49,52,2,7],
  ['lu','Luxembourg',49,50,5,7],['ch','Switzerland',45,48,6,11],
  ['at','Austria',46,49,9,17],['it','Italy',36,47,6,19],
  ['es','Spain',36,44,-10,4],['pt','Portugal',37,42,-10,-6],
  ['pl','Poland',49,55,14,24],['cz','Czechia',48,51,12,19],
  ['sk','Slovakia',47,50,17,23],['hu','Hungary',45,49,16,23],
  ['ro','Romania',43,48,20,30],['bg','Bulgaria',41,44,22,29],
  ['gr','Greece',35,42,19,30],['hr','Croatia',42,47,13,20],
  ['rs','Serbia',42,46,19,23],['ba','Bosnia',43,45,15,20],
  ['si','Slovenia',45,47,13,17],['al','Albania',39,43,19,21],
  ['mk','N.Macedonia',40,42,20,23],['me','Montenegro',41,44,18,21],
  ['se','Sweden',55,69,11,24],['no','Norway',58,71,4,31],
  ['dk','Denmark',54,58,8,15],['fi','Finland',60,70,20,32],
  ['ee','Estonia',57,60,21,28],['lv','Latvia',55,58,21,28],
  ['lt','Lithuania',53,57,21,27],['ua','Ukraine',44,53,22,40],
  ['md','Moldova',46,49,27,30],['by','Belarus',51,57,23,33],
  ['tr','Turkey',36,42,26,45],['cy','Cyprus',34,36,32,35],
  ['il','Israel',29,34,34,36],['ae','UAE',22,26,51,56],
  ['sa','Saudi Arabia',16,32,34,56],['qa','Qatar',24,27,50,52],
  ['kw','Kuwait',28,31,46,49],['om','Oman',16,27,52,60],
  // Then Russia
  ['ru','Russia',41,82,27,180],
  // Americas - specific before large
  ['us','United States',24,50,-125,-66],['ca','Canada',42,72,-141,-52],
  ['mx','Mexico',14,33,-118,-86],['gt','Guatemala',13,18,-92,-88],
  ['cr','Costa Rica',8,11,-86,-82],['pa','Panama',7,10,-83,-77],
  ['co','Colombia',-5,14,-80,-66],['ve','Venezuela',0,13,-74,-59],
  ['ec','Ecuador',-5,2,-81,-75],['pe','Peru',-19,0,-82,-68],
  ['br','Brazil',-34,6,-74,-35],['ar','Argentina',-55,-21,-74,-53],
  ['cl','Chile',-56,-17,-76,-66],['uy','Uruguay',-35,-30,-59,-53],
  // Africa/Oceania
  ['za','South Africa',-35,-22,16,33],['eg','Egypt',22,32,24,37],
  ['ma','Morocco',27,36,-13,0],['ng','Nigeria',4,14,2,15],
  ['ke','Kenya',-5,5,34,42],['tz','Tanzania',-12,-1,29,41],
  ['au','Australia',-44,-10,113,154],['nz','New Zealand',-47,-34,166,179],
];
function bboxLookup(lat, lon) {
  for (var i = 0; i < CC.length; i++) {
    var c = CC[i];
    if (lat >= c[2] && lat <= c[3] && lon >= c[4] && lon <= c[5]) return { cc: c[0], country: c[1], city: '' };
  }
  return { cc: '', country: 'Unknown', city: '' };
}

// Country name to ISO code mapping for Photon API results
var nameToCC = {"Afghanistan":"af","Albania":"al","Algeria":"dz","Argentina":"ar","Armenia":"am","Australia":"au","Austria":"at","Azerbaijan":"az","Bahrain":"bh","Bangladesh":"bd","Belarus":"by","Belgium":"be","Bolivia":"bo","Bosnia and Herzegovina":"ba","Brazil":"br","Bulgaria":"bg","Cambodia":"kh","Canada":"ca","Chile":"cl","China":"cn","Colombia":"co","Costa Rica":"cr","Croatia":"hr","Cuba":"cu","Cyprus":"cy","Czech Republic":"cz","Czechia":"cz","Denmark":"dk","Dominican Republic":"do","Ecuador":"ec","Egypt":"eg","El Salvador":"sv","Estonia":"ee","Ethiopia":"et","Finland":"fi","France":"fr","Georgia":"ge","Germany":"de","Ghana":"gh","Greece":"gr","Guatemala":"gt","Honduras":"hn","Hong Kong":"hk","Hungary":"hu","Iceland":"is","India":"in","Indonesia":"id","Iran":"ir","Iraq":"iq","Ireland":"ie","Israel":"il","Italy":"it","Jamaica":"jm","Japan":"jp","Jordan":"jo","Kazakhstan":"kz","Kenya":"ke","Kuwait":"kw","Latvia":"lv","Lebanon":"lb","Libya":"ly","Lithuania":"lt","Luxembourg":"lu","Malaysia":"my","Malta":"mt","Mexico":"mx","Moldova":"md","Mongolia":"mn","Montenegro":"me","Morocco":"ma","Myanmar":"mm","Nepal":"np","Netherlands":"nl","New Zealand":"nz","Nigeria":"ng","North Macedonia":"mk","Norway":"no","Oman":"om","Pakistan":"pk","Panama":"pa","Paraguay":"py","Peru":"pe","Philippines":"ph","Poland":"pl","Portugal":"pt","Qatar":"qa","Romania":"ro","Russia":"ru","Saudi Arabia":"sa","Senegal":"sn","Serbia":"rs","Singapore":"sg","Slovakia":"sk","Slovenia":"si","South Africa":"za","South Korea":"kr","Spain":"es","Sri Lanka":"lk","Sweden":"se","Switzerland":"ch","Taiwan":"tw","Tanzania":"tz","Thailand":"th","Tunisia":"tn","Turkey":"tr","Türkiye":"tr","Ukraine":"ua","United Arab Emirates":"ae","United Kingdom":"gb","United States":"us","United States of America":"us","Uruguay":"uy","Uzbekistan":"uz","Venezuela":"ve","Vietnam":"vn","Viet Nam":"vn"};

// Photon API queue
var apiQ = [], apiBusy = false;
function processApi() {
  if (apiBusy || !apiQ.length) return;
  apiBusy = true;
  var item = apiQ.shift();
  fetch('https://photon.komoot.io/reverse?lon=' + item.lon.toFixed(4) + '&lat=' + item.lat.toFixed(4))
    .then(function(r) { return r.json(); })
    .then(function(d) {
      var p = (d.features && d.features[0]) ? d.features[0].properties : {};
      var country = p.country || 'Unknown';
      geoCache[item.uuid] = { cc: nameToCC[country] || '', country: country, city: p.city || p.state || '' };
      saveGeo();
    })
    .catch(function() {})
    .finally(function() { apiBusy = false; setTimeout(processApi, 200); });
}

function flagHtml(cc) {
  if (!cc) return '';
  return '<span class="fi fi-' + cc + '" style="margin-right:4px"></span>';
}

// Returns {html} for popup. Checks cache, falls back to bbox, queues API.
function geo(uuid, lat, lon) {
  var g = geoCache[uuid];
  if (g) return flagHtml(g.cc) + '<b>' + g.country + '</b>' + (g.city ? ' <span style="opacity:.7">' + g.city + '</span>' : '');
  // Queue API
  if (!apiQ.some(function(q) { return q.uuid === uuid; })) {
    apiQ.push({ uuid: uuid, lat: lat, lon: lon });
    if (!apiBusy) setTimeout(processApi, 10);
  }
  // Bbox fallback
  var bb = bboxLookup(lat, lon);
  return flagHtml(bb.cc) + '<b>' + bb.country + '</b>';
}

// === LAYERS - IDENTICAL TO WORKING VERSION ===

function init() {
  if (typeof OLMap === 'undefined' || !OLMap) { setTimeout(init, 500); return; }

  var lG = new ol.layer.Vector({ source: srcG, zIndex: 48, visible: false,
    style: new ol.style.Style({image: new ol.style.Circle({radius:6,
      fill: new ol.style.Fill({color:'rgba(46,213,115,0.9)'}),
      stroke: new ol.style.Stroke({color:'rgba(46,213,115,0.3)',width:10})
    })})
  });
  var lO = new ol.layer.Vector({ source: srcO, zIndex: 47, visible: false,
    style: new ol.style.Style({image: new ol.style.Circle({radius:5,
      fill: new ol.style.Fill({color:'rgba(255,165,2,0.85)'}),
      stroke: new ol.style.Stroke({color:'rgba(255,165,2,0.2)',width:7})
    })})
  });
  var lR = new ol.layer.Vector({ source: srcR, zIndex: 46, visible: false,
    style: new ol.style.Style({image: new ol.style.Circle({radius:4,
      fill: new ol.style.Fill({color:'rgba(255,71,87,0.7)'}),
      stroke: new ol.style.Stroke({color:'rgba(255,71,87,0.15)',width:5})
    })})
  });
  var lX = new ol.layer.Vector({ source: srcX, zIndex: 45, visible: false,
    style: new ol.style.Style({image: new ol.style.Circle({radius:3,
      fill: new ol.style.Fill({color:'rgba(150,150,150,0.4)'}),
      stroke: new ol.style.Stroke({color:'rgba(150,150,150,0.1)',width:3})
    })})
  });

  layers = [lG, lO, lR, lX];

  var toggle = new ol.layer.Vector({
    name: 'receivers_toggle', type: 'overlay', title: '\u{1F4E1} Receivers',
    source: new ol.source.Vector(), zIndex: 44, visible: false
  });

  OLMap.addLayer(lX); OLMap.addLayer(lR); OLMap.addLayer(lO); OLMap.addLayer(lG);
  OLMap.addLayer(toggle);
  OLMap.getControls().forEach(function(c){if(c.renderPanel)c.renderPanel()});

  var popupEl = document.createElement('div');
  popupEl.style.cssText = 'background:#111827ee;color:#c8d6e5;padding:8px 12px;border-radius:6px;border:1px solid #1e3a5f;font-size:12px;line-height:1.5;pointer-events:none;white-space:nowrap;box-shadow:0 4px 12px rgba(0,0,0,.5)';
  var popup = new ol.Overlay({element:popupEl,positioning:'bottom-center',offset:[0,-10],stopEvent:false});
  OLMap.addOverlay(popup);

  OLMap.on('pointermove', function(evt) {
    if (!toggle.getVisible()) {popup.setPosition(undefined);return}
    var hit = OLMap.forEachFeatureAtPixel(evt.pixel, function(f){return f.idx!==undefined?f:null});
    if (hit && allData[hit.idx]) {
      var r = allData[hit.idx];
      popupEl.innerHTML = geo(r[0], r[8], r[9])+
        '<br><span style="color:#00d4ff;font-size:11px">'+r[0]+'</span>'+
        '<br>Rate: <b style="color:#2ed573">'+r[1].toFixed(1)+'</b> pos/s \u00b7 T/O: '+r[2].toFixed(1)+'/hr'+
        '<br>Range: '+(r[4]-r[3]).toFixed(1)+'\u00b0\u00d7'+(r[6]-r[5]).toFixed(1)+'\u00b0'+
        (r[7]?'<br><span style="color:#ff4757">\u26a0 Bad</span>':'');
      popup.setPosition(evt.coordinate);
    } else {popup.setPosition(undefined)}
  });

  toggle.on('change:visible', function() {
    var vis = toggle.getVisible();
    for (var i=0;i<layers.length;i++) layers[i].setVisible(vis);
    if (vis) {
      doFetch();
      if (!refreshTimer) refreshTimer = setInterval(doFetch, 30000);
    } else {
      clearInterval(refreshTimer); refreshTimer=null;
      popup.setPosition(undefined);
      srcG.clear();srcO.clear();srcR.clear();srcX.clear();allData=[];
    }
  });
  console.log('receivers-overlay: ready, geo cache: '+Object.keys(geoCache).length);
}

function doFetch() {
  fetch('data/receivers.json').then(function(r){return r.json()}).then(function(d) {
    var recs = d.receivers || [];
    allData = recs;
    srcG.clear();srcO.clear();srcR.clear();srcX.clear();
    for (var i = 0; i < recs.length; i++) {
      var r = recs[i];
      var f = new ol.Feature(new ol.geom.Point(ol.proj.fromLonLat([r[9], r[8]])));
      f.idx = i;
      var rate = r[1];
      if (rate > 10) srcG.addFeature(f);
      else if (rate > 3) srcO.addFeature(f);
      else if (rate > 0.5) srcR.addFeature(f);
      else srcX.addFeature(f);
    }
    console.log('receivers-overlay: '+recs.length+' plotted');
  }).catch(function(e){console.error(e)});
}

init();
})();
