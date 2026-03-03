/**
 * ScrcpyClient — H.264 hardware decoding via WebCodecs + binary WebSocket control
 *
 * Usage:
 *   var client = new ScrcpyClient(udid, canvas);
 *   client.connect()
 *     .then(function() { console.log('connected'); })
 *     .catch(function(e) { console.log('failed', e); });
 *
 * Touch: client.sendTouch(action, x, y, screenW, screenH, pressure)
 * Key:   client.sendKey(action, keycode)
 * Stop:  client.disconnect()
 */

class ScrcpyClient {
    constructor(udid, canvas) {
        this.udid = udid;
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.ws = null;
        this.decoder = null;
        this.connected = false;
        this.width = 0;
        this.height = 0;
        this.codecString = 'avc1.640028'; // default H.264 High profile

        // FPS tracking
        this.frameCount = 0;
        this.fps = 0;
        this._fpsTimer = null;

        // Callbacks
        this.onInit = null;
        this.onFrame = null;
        this.onDisconnect = null;
        this.onError = null;

        // Pending SPS for codec string extraction
        this._pendingSPS = null;
    }

    /**
     * Check if WebCodecs is supported
     */
    static isSupported() {
        return typeof VideoDecoder !== 'undefined' && typeof EncodedVideoChunk !== 'undefined';
    }

    /**
     * Connect to scrcpy WebSocket and start decoding
     */
    connect() {
        var self = this;
        return new Promise(function(resolve, reject) {
            if (!ScrcpyClient.isSupported()) {
                reject(new Error('WebCodecs not supported in this browser'));
                return;
            }

            var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
            var url = protocol + '//' + location.host + '/scrcpy/' + self.udid + '/ws';
            console.log('[Scrcpy] Connecting:', url);

            self.ws = new WebSocket(url);
            self.ws.binaryType = 'arraybuffer';

            var resolved = false;

            self.ws.onopen = function() {
                console.log('[Scrcpy] WebSocket connected');
            };

            self.ws.onmessage = function(event) {
                if (typeof event.data === 'string') {
                    // JSON text message (init or error)
                    var msg = JSON.parse(event.data);
                    if (msg.type === 'init') {
                        self.width = msg.width;
                        self.height = msg.height;
                        console.log('[Scrcpy] Init: ' + msg.codec + ' ' + msg.width + 'x' + msg.height);
                        self._initDecoder();
                        self.connected = true;
                        self._startFpsCounter();
                        if (self.onInit) self.onInit(msg);
                        if (!resolved) {
                            resolved = true;
                            resolve();
                        }
                    } else if (msg.type === 'error') {
                        console.error('[Scrcpy] Server error:', msg.message);
                        if (!resolved) {
                            resolved = true;
                            reject(new Error(msg.message));
                        }
                    }
                } else {
                    // Binary frame data
                    self._onBinaryFrame(new Uint8Array(event.data));
                }
            };

            self.ws.onerror = function(e) {
                console.error('[Scrcpy] WebSocket error:', e);
                if (!resolved) {
                    resolved = true;
                    reject(new Error('WebSocket connection failed'));
                }
            };

            self.ws.onclose = function() {
                console.log('[Scrcpy] WebSocket closed');
                self.connected = false;
                self._stopFpsCounter();
                if (self.decoder) {
                    try { self.decoder.close(); } catch(e) {}
                    self.decoder = null;
                }
                if (self.onDisconnect) self.onDisconnect();
            };

            // Timeout — scrcpy-server needs time to start (Java app_process)
            setTimeout(function() {
                if (!resolved) {
                    resolved = true;
                    self.disconnect();
                    reject(new Error('Connection timeout'));
                }
            }, 15000);
        });
    }

    /**
     * Disconnect and clean up
     */
    disconnect() {
        this.connected = false;
        this._stopFpsCounter();
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        if (this.decoder) {
            try { this.decoder.close(); } catch(e) {}
            this.decoder = null;
        }
    }

    /**
     * Initialize the VideoDecoder
     */
    _initDecoder() {
        var self = this;

        self.decoder = new VideoDecoder({
            output: function(frame) {
                self._renderFrame(frame);
            },
            error: function(e) {
                console.error('[Scrcpy] Decoder error:', e);
                if (self.onError) self.onError(e);
            }
        });

        // Configure with default codec string; will reconfigure after SPS parse
        self.decoder.configure({
            codec: self.codecString,
            optimizeForLatency: true,
        });

        console.log('[Scrcpy] VideoDecoder initialized');
    }

    /**
     * Process a binary frame from the WebSocket
     *
     * Format: flags(1B) + size(4B BE) + NAL data
     * flags: bit0 = config (SPS/PPS), bit1 = keyframe
     */
    _onBinaryFrame(data) {
        if (data.length < 5) return;

        var flags = data[0];
        var size = (data[1] << 24) | (data[2] << 16) | (data[3] << 8) | data[4];
        var nalData = data.subarray(5, 5 + size);

        if (nalData.length !== size) return;

        var isConfig = (flags & 1) !== 0;
        var isKey = (flags & 2) !== 0;

        if (!this.decoder || this.decoder.state === 'closed') return;

        // Always try to extract codec string from SPS (some encoders embed
        // SPS/PPS in the first keyframe without setting the config flag)
        if (isConfig || isKey) {
            var codecStr = this._parseCodecString(nalData);
            if (codecStr && codecStr !== this.codecString) {
                this.codecString = codecStr;
                console.log('[Scrcpy] Codec string from ' + (isConfig ? 'config' : 'keyframe') + ':', codecStr);
                try {
                    this.decoder.configure({
                        codec: codecStr,
                        optimizeForLatency: true,
                    });
                } catch(e) {
                    console.warn('[Scrcpy] Reconfigure failed:', e);
                }
            }
        }

        if (isConfig && !isKey) {
            // Pure config packet (SPS/PPS only, no IDR) — decode as description
            try {
                var chunk = new EncodedVideoChunk({
                    type: 'key',
                    timestamp: 0,
                    data: nalData,
                });
                this.decoder.decode(chunk);
            } catch(e) {
                // Config packets may not always be decodable alone
            }
            return;
        }

        try {
            var chunk = new EncodedVideoChunk({
                type: isKey ? 'key' : 'delta',
                timestamp: performance.now() * 1000, // microseconds
                data: nalData,
            });
            this.decoder.decode(chunk);
        } catch(e) {
            // Skip frames that can't be decoded
        }
    }

    /**
     * Render a decoded video frame to the canvas
     */
    _renderFrame(frame) {
        // Update canvas size if needed
        if (this.canvas.width !== frame.displayWidth || this.canvas.height !== frame.displayHeight) {
            this.canvas.width = frame.displayWidth;
            this.canvas.height = frame.displayHeight;
            this.width = frame.displayWidth;
            this.height = frame.displayHeight;
        }

        this.ctx.drawImage(frame, 0, 0);
        frame.close();

        this.frameCount++;
        if (this.onFrame) this.onFrame();
    }

    /**
     * Parse SPS NAL unit to extract avc1.XXYYZZ codec string
     */
    _parseCodecString(nalData) {
        // Find SPS NAL unit (type 7) in the data
        // NAL units are separated by start codes 00 00 00 01 or 00 00 01
        var i = 0;
        while (i < nalData.length - 4) {
            // Look for start code
            if (nalData[i] === 0 && nalData[i+1] === 0) {
                var startCodeLen = 0;
                if (nalData[i+2] === 0 && nalData[i+3] === 1) {
                    startCodeLen = 4;
                } else if (nalData[i+2] === 1) {
                    startCodeLen = 3;
                }

                if (startCodeLen > 0) {
                    var nalType = nalData[i + startCodeLen] & 0x1F;
                    if (nalType === 7 && i + startCodeLen + 3 < nalData.length) {
                        // SPS found: profile_idc, constraint_set_flags, level_idc
                        var profile = nalData[i + startCodeLen + 1];
                        var compat = nalData[i + startCodeLen + 2];
                        var level = nalData[i + startCodeLen + 3];
                        return 'avc1.' +
                            profile.toString(16).padStart(2, '0') +
                            compat.toString(16).padStart(2, '0') +
                            level.toString(16).padStart(2, '0');
                    }
                    i += startCodeLen + 1;
                    continue;
                }
            }
            i++;
        }

        // Try without start code (raw NAL)
        if (nalData.length > 3) {
            var nalType = nalData[0] & 0x1F;
            if (nalType === 7) {
                var profile = nalData[1];
                var compat = nalData[2];
                var level = nalData[3];
                return 'avc1.' +
                    profile.toString(16).padStart(2, '0') +
                    compat.toString(16).padStart(2, '0') +
                    level.toString(16).padStart(2, '0');
            }
        }

        return null;
    }

    /**
     * Send a touch event (28 bytes binary)
     *
     * @param {number} action - 0=down, 1=up, 2=move
     * @param {number} x - device x coordinate
     * @param {number} y - device y coordinate
     * @param {number} screenW - device screen width
     * @param {number} screenH - device screen height
     * @param {number} pressure - 0xFFFF for press, 0 for release
     */
    sendTouch(action, x, y, screenW, screenH, pressure) {
        if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;

        var buf = new ArrayBuffer(28);
        var view = new DataView(buf);

        view.setUint8(0, 2);           // type = INJECT_TOUCH_EVENT
        view.setUint8(1, action);       // action
        // pointer_id = -1 (POINTER_ID_MOUSE)
        view.setUint32(2, 0xFFFFFFFF, false);
        view.setUint32(6, 0xFFFFFFFF, false);
        view.setUint32(10, x, false);   // x
        view.setUint32(14, y, false);   // y
        view.setUint16(18, screenW, false);  // width
        view.setUint16(20, screenH, false);  // height
        view.setUint16(22, pressure, false); // pressure
        view.setUint32(24, 0, false);   // action_button + buttons

        this.ws.send(buf);
    }

    /**
     * Send a key event (14 bytes binary)
     *
     * @param {number} action - 0=down, 1=up
     * @param {number} keycode - Android KEYCODE_*
     */
    sendKey(action, keycode) {
        if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;

        var buf = new ArrayBuffer(14);
        var view = new DataView(buf);

        view.setUint8(0, 0);           // type = INJECT_KEYCODE
        view.setUint8(1, action);       // action
        view.setUint32(2, keycode, false); // keycode
        view.setUint32(6, 0, false);    // repeat
        view.setUint32(10, 0, false);   // metastate

        this.ws.send(buf);
    }

    /**
     * Convenience: send click (down + up) at device coordinates
     */
    sendClick(x, y, screenW, screenH) {
        this.sendTouch(0, x, y, screenW, screenH, 0xFFFF); // down
        var self = this;
        setTimeout(function() {
            self.sendTouch(1, x, y, screenW, screenH, 0); // up
        }, 20);
    }

    /**
     * Convenience: send key press (down + up)
     */
    sendKeyPress(keycode) {
        this.sendKey(0, keycode); // down
        var self = this;
        setTimeout(function() {
            self.sendKey(1, keycode); // up
        }, 20);
    }

    _startFpsCounter() {
        var self = this;
        this._fpsTimer = setInterval(function() {
            self.fps = self.frameCount;
            self.frameCount = 0;
        }, 1000);
    }

    _stopFpsCounter() {
        if (this._fpsTimer) {
            clearInterval(this._fpsTimer);
            this._fpsTimer = null;
        }
        this.fps = 0;
        this.frameCount = 0;
    }
}

// Android keycodes for convenience
ScrcpyClient.KEYCODE = {
    HOME: 3,
    BACK: 4,
    POWER: 26,
    MENU: 82,
    ENTER: 66,
    DEL: 67,
    FORWARD_DEL: 112,
    TAB: 61,
    DPAD_UP: 19,
    DPAD_DOWN: 20,
    DPAD_LEFT: 21,
    DPAD_RIGHT: 22,
    WAKEUP: 224,
    VOLUME_UP: 24,
    VOLUME_DOWN: 25,
    APP_SWITCH: 187,
};

window.ScrcpyClient = ScrcpyClient;
