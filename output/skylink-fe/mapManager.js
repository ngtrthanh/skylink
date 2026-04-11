/**
 * MapManager - Consolidated MapLibre GL JS Initializer
 * Combines all map initialization logic with style management, controls, and projection toggle
 */

class MapManager {
    static #instance = null;
    static #API_KEYS = {
        jawg: "qML7E6HmMKb6LQJgxHedkuht58y48dIpzawFGfCXdHzqnZWQlscx5zmyw7uYgTZG",
        maptiler: ["nRYox0R1ZyZ6XqSStq4S", "yShyGLZC3JMFFIUecAOl"]
    };

    #map = null;
    #currentStyle = 'MapTiler Street Dark';
    #currentProjection = 'globe';
    #keyIndex = 0;
    #controls = {};
    #styleSelectEl = null;
    #projectionBtn = null;

    constructor(options = {}) {
        if (MapManager.#instance) return MapManager.#instance;

        this.containerId = options.containerId || 'maplibreMap';
        this.center = options.center || [6, 34];
        this.zoom = options.zoom ?? 2;

        // Restore saved preferences
        this.#loadPreferences();

        this.#initMap();
        MapManager.#instance = this;
    }

    #loadPreferences() {
        try {
            const savedStyle = localStorage.getItem('mapStylePreference');
            const savedProjection = localStorage.getItem('mapProjection');
            if (savedStyle && this.#styles[savedStyle]) {
                this.#currentStyle = savedStyle;
            }
            if (savedProjection && ['globe', 'mercator'].includes(savedProjection)) {
                this.#currentProjection = savedProjection;
            }
        } catch (e) {
            console.warn('Could not load preferences from localStorage:', e);
        }
    }

    #savePreference(key, value) {
        try {
            localStorage.setItem(key, value);
        } catch (e) {
            console.warn('Could not save preference:', e);
        }
    }

    #styles = {
        // Jawg Styles
        'Jawg Light': { type: 'jawg', id: 'jawg-light' },
        'Jawg Dark': { type: 'jawg', id: 'jawg-dark' },
        'Jawg Streets': { type: 'jawg', id: 'jawg-streets' },
        'Jawg Terrain': { type: 'jawg', id: 'jawg-terrain' },
        'Jawg Lagoon': { type: 'jawg', id: 'jawg-lagoon' },
        'Jawg Matrix': { type: 'jawg', id: 'jawg-matrix' },
        'Jawg Sunny': { type: 'jawg', id: 'jawg-sunny' },

        // MapTiler Styles
        'MapTiler Street': { type: 'maptiler', url: 'https://api.maptiler.com/maps/streets/style.json' },
        'MapTiler Street Dark': { type: 'maptiler', url: 'https://api.maptiler.com/maps/streets-v2-dark/style.json' },
        'MapTiler Ocean': { type: 'maptiler', url: 'https://api.maptiler.com/maps/ocean/style.json' },
        'MapTiler Backdrop': { type: 'maptiler', url: 'https://api.maptiler.com/maps/backdrop/style.json' },
        'MapTiler OSM': { type: 'maptiler', url: 'https://api.maptiler.com/maps/openstreetmap/style.json' },
        'MapTiler Satellite': { type: 'maptiler', url: 'https://api.maptiler.com/maps/satellite/style.json' },

        // ESRI Style
        'ESRI World Imagery': {
            type: 'raster',
            config: {
                url: 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}',
                attribution: '© Esri — Source: Esri and partners'
            }
        },

        // OpenFreeMap Styles
        'OpenFreeMap Liberty': 'https://tiles.openfreemap.org/styles/liberty',
        'OpenFreeMap Positron': 'https://tiles.openfreemap.org/styles/positron',
        'OpenFreeMap Bright': 'https://tiles.openfreemap.org/styles/bright',
        'OpenFreeMap Dark': 'https://tiles.openfreemap.org/styles/dark',

        // Demo Style
        'Demo': 'https://demotiles.maplibre.org/style.json'
    };

    #initMap() {
        const styleUrl = this.#getStyleUrl(this.#styles[this.#currentStyle]);

        this.#map = new maplibregl.Map({
            container: this.containerId,
            style: styleUrl,
            center: this.center,
            zoom: this.zoom,
            antialias: true,
            attributionControl: false,
            hash: true
        });

        // Wait for style to load before applying projection
        this.#map.on('style.load', () => {
            this.#applyProjection();
            this.#updateUIState();
        });

        this.#setupControls();
        this.#setupEventListeners();
    }

    #getStyleUrl(style) {
        if (!style) return this.#styles['Demo'];

        switch (style.type) {
            case 'jawg':
                return `https://api.jawg.io/styles/${style.id}.json?access-token=${MapManager.#API_KEYS.jawg}`;
            case 'maptiler':
                return `${style.url}?key=${this.#getNextMaptilerKey()}`;
            case 'raster':
                return this.#createRasterStyle(style.config);
            default:
                return typeof style === 'string' ? style : style.url;
        }
    }

    #createRasterStyle(config) {
        return {
            version: 8,
            sources: {
                'raster-tiles': {
                    type: 'raster',
                    tiles: [config.url],
                    tileSize: 256,
                    attribution: config.attribution
                }
            },
            layers: [{
                id: 'raster-tiles-layer',
                type: 'raster',
                source: 'raster-tiles',
                minzoom: 0,
                maxzoom: 22
            }]
        };
    }

    #getNextMaptilerKey() {
        const keys = MapManager.#API_KEYS.maptiler;
        const key = keys[this.#keyIndex];
        this.#keyIndex = (this.#keyIndex + 1) % keys.length;
        return key;
    }

    #setupControls() {
        this.#controls.navigation = new maplibregl.NavigationControl();
        this.#controls.scale = new maplibregl.ScaleControl({ maxWidth: 150, unit: 'metric' });
        this.#controls.fullscreen = new maplibregl.FullscreenControl();

        this.#map.addControl(this.#controls.navigation, 'bottom-right');
        this.#map.addControl(this.#controls.scale, 'bottom-left');
        this.#map.addControl(this.#controls.fullscreen, 'bottom-right');

        this.#addStyleSelector();
        this.#addProjectionToggle();
        this.#addAttribution();
    }

    #addAttribution() {
        const attr = document.createElement('div');
        attr.className = 'map-attribution maplibre-ctrl';
        attr.innerHTML = '© <a href="https://www.openstreetmap.org/copyright" target="_blank" rel="noopener">OpenStreetMap</a> contributors';
        this.#map.getContainer().appendChild(attr);
    }

    #addStyleSelector() {
        const container = document.createElement('div');
        container.className = 'maplibre-ctrl maplibre-ctrl-group style-selector-container';

        const select = document.createElement('select');
        select.className = 'map-style-select';
        select.title = 'Change map style';
        select.id = 'map-style-select';

        const styleGroups = {
            'Jawg Maps': ['Jawg Light', 'Jawg Dark', 'Jawg Streets', 'Jawg Terrain', 'Jawg Lagoon', 'Jawg Matrix', 'Jawg Sunny'],
            'MapTiler': ['MapTiler Street', 'MapTiler Street Dark', 'MapTiler Ocean', 'MapTiler Backdrop', 'MapTiler OSM', 'MapTiler Satellite'],
            'Other': ['ESRI World Imagery', 'OpenFreeMap Liberty', 'OpenFreeMap Positron', 'OpenFreeMap Bright', 'OpenFreeMap Dark', 'Demo']
        };

        Object.entries(styleGroups).forEach(([groupName, styles]) => {
            const group = document.createElement('optgroup');
            group.label = groupName;
            styles.forEach(styleName => {
                if (this.#styles[styleName]) {
                    const option = document.createElement('option');
                    option.value = styleName;
                    option.textContent = styleName;
                    option.selected = styleName === this.#currentStyle;
                    group.appendChild(option);
                }
            });
            select.appendChild(group);
        });

        select.addEventListener('change', (e) => this.changeStyle(e.target.value));
        container.appendChild(select);
        this.#map.getContainer().appendChild(container);
        this.#styleSelectEl = select;
    }

    #addProjectionToggle() {
        const container = document.createElement('div');
        container.className = 'maplibre-ctrl maplibre-ctrl-group projection-toggle-container';

        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'projection-toggle';
        btn.title = 'Toggle projection (Globe / Mercator)';
        btn.textContent = this.#currentProjection === 'globe' ? 'Globe' : 'Mercator';
        btn.setAttribute('aria-pressed', String(this.#currentProjection === 'globe'));
        btn.id = 'projection-toggle';

        btn.addEventListener('click', (e) => {
            e.preventDefault();
            const next = this.#currentProjection === 'globe' ? 'mercator' : 'globe';
            this.setProjection(next);
        });

        container.appendChild(btn);
        this.#map.getContainer().appendChild(container);
        this.#projectionBtn = btn;
    }

    #applyProjection() {
        if (!this.#map) return;

        try {
            this.#map.setProjection({ type: this.#currentProjection });
            this.#updateUIState();
        } catch (err) {
            console.warn('Failed to apply projection:', err);
        }
    }

    setProjection(projection) {
        if (!['globe', 'mercator'].includes(projection)) return;

        this.#currentProjection = projection;
        this.#savePreference('mapProjection', projection);

        // Update button text
        if (this.#projectionBtn) {
            this.#projectionBtn.textContent = projection === 'globe' ? 'Globe' : 'Mercator';
            this.#projectionBtn.setAttribute('aria-pressed', String(projection === 'globe'));
        }

        this.#applyProjection();
    }

    changeStyle(styleName) {
        if (!this.#map || !this.#styles[styleName]) return;

        try {
            const style = this.#styles[styleName];
            this.#map.setStyle(this.#getStyleUrl(style));
            this.#currentStyle = styleName;
            this.#savePreference('mapStylePreference', styleName);

            if (this.#styleSelectEl) {
                this.#styleSelectEl.value = styleName;
            }
        } catch (error) {
            console.error('Style change failed:', error);
        }
    }

    #updateUIState() {
        if (this.#projectionBtn) {
            this.#projectionBtn.disabled = false;
            this.#projectionBtn.textContent = this.#currentProjection === 'globe' ? 'Globe' : 'Mercator';
            this.#projectionBtn.setAttribute('aria-pressed', String(this.#currentProjection === 'globe'));
        }
        if (this.#styleSelectEl) {
            this.#styleSelectEl.value = this.#currentStyle;
        }
    }

    #setupEventListeners() {
        // Responsive controls with debounce
        let resizeTimer;
        const adjustControls = () => {
            const isMobile = window.innerWidth < 768;

            // Reposition controls
            this.#map.removeControl(this.#controls.navigation);
            this.#map.removeControl(this.#controls.fullscreen);
            this.#map.removeControl(this.#controls.scale);

            this.#controls.navigation = new maplibregl.NavigationControl();
            this.#controls.fullscreen = new maplibregl.FullscreenControl();
            this.#controls.scale = new maplibregl.ScaleControl({
                maxWidth: isMobile ? 100 : 150,
                unit: 'metric'
            });

            this.#map.addControl(this.#controls.navigation, isMobile ? 'top-left' : 'bottom-right');
            this.#map.addControl(this.#controls.fullscreen, isMobile ? 'top-left' : 'bottom-right');
            this.#map.addControl(this.#controls.scale, 'bottom-left');
        };

        window.addEventListener('resize', () => {
            clearTimeout(resizeTimer);
            resizeTimer = setTimeout(adjustControls, 150);
        });

        // Initial adjustment
        adjustControls();
    }

    get map() {
        return this.#map;
    }

    addMarker(coordinates, options = {}) {
        if (!this.#map) return null;

        const marker = new maplibregl.Marker(options)
            .setLngLat(coordinates)
            .addTo(this.#map);

        return marker;
    }

    removeMarker(marker) {
        if (marker) {
            marker.remove();
        }
    }

    flyTo(coordinates, options = {}) {
        if (!this.#map) return;

        this.#map.flyTo({
            center: coordinates,
            zoom: options.zoom ?? this.#map.getZoom(),
            ...options
        });
    }

    destroy() {
        if (this.#map) {
            this.#map.remove();
            this.#map = null;
        }
        MapManager.#instance = null;
    }
}

// Public interface
function initMap(options = {}) {
    const mapManager = new MapManager(options);
    return mapManager.map;
}

// Export for module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { MapManager, initMap };
}
