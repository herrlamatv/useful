/* ai chat exporter - by herrlamatv
 * Kleiner ZIP-Schreiber ohne Bibliothek: Deflate macht der Browser selbst
 * (CompressionStream 'deflate-raw'), CRC32 und Header schreiben wir.
 * Kein ZIP64 - reicht für Chat-Exporte locker (bis 4 GB, 65535 Dateien).
 */

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

export function crc32(bytes) {
  let c = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) c = CRC_TABLE[(c ^ bytes[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

async function deflateRaw(bytes) {
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate-raw'));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

function dosDateTime(date) {
  const d = date instanceof Date && !isNaN(date) ? date : new Date();
  const year = Math.max(1980, d.getFullYear());
  return {
    time: (d.getHours() << 11) | (d.getMinutes() << 5) | Math.floor(d.getSeconds() / 2),
    date: ((year - 1980) << 9) | ((d.getMonth() + 1) << 5) | d.getDate()
  };
}

const encoder = new TextEncoder();

async function toBytes(data) {
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (typeof Blob !== 'undefined' && data instanceof Blob) return new Uint8Array(await data.arrayBuffer());
  return encoder.encode(String(data == null ? '' : data));
}

export class ZipWriter {
  constructor({ compress = true } = {}) {
    this.compress = compress && typeof CompressionStream !== 'undefined';
    this.parts = [];
    this.central = [];
    this.offset = 0;
    this.paths = new Set();
  }

  /** Pfad eindeutig machen: "a.md" -> "a (2).md". */
  unique(path) {
    if (!this.paths.has(path.toLowerCase())) return path;
    const dot = path.lastIndexOf('.');
    const slash = path.lastIndexOf('/');
    const stem = dot > slash ? path.slice(0, dot) : path;
    const ext = dot > slash ? path.slice(dot) : '';
    let n = 2;
    while (this.paths.has((stem + ' (' + n + ')' + ext).toLowerCase())) n++;
    return stem + ' (' + n + ')' + ext;
  }

  async add(path, data, date = new Date()) {
    const name = this.unique(String(path).replace(/\\/g, '/').replace(/^\/+/, ''));
    this.paths.add(name.toLowerCase());
    const raw = await toBytes(data);
    const crc = crc32(raw);
    let body = raw;
    let method = 0;
    if (this.compress && raw.length > 64) {
      const packed = await deflateRaw(raw);
      if (packed.length < raw.length) {
        body = packed;
        method = 8;
      }
    }
    const nameBytes = encoder.encode(name);
    const { time, date: day } = dosDateTime(date);

    const local = new DataView(new ArrayBuffer(30));
    local.setUint32(0, 0x04034b50, true);
    local.setUint16(4, 20, true);
    local.setUint16(6, 0x0800, true); // Dateinamen in UTF-8
    local.setUint16(8, method, true);
    local.setUint16(10, time, true);
    local.setUint16(12, day, true);
    local.setUint32(14, crc, true);
    local.setUint32(18, body.length, true);
    local.setUint32(22, raw.length, true);
    local.setUint16(26, nameBytes.length, true);
    local.setUint16(28, 0, true);

    const cen = new DataView(new ArrayBuffer(46));
    cen.setUint32(0, 0x02014b50, true);
    cen.setUint16(4, 20, true);
    cen.setUint16(6, 20, true);
    cen.setUint16(8, 0x0800, true);
    cen.setUint16(10, method, true);
    cen.setUint16(12, time, true);
    cen.setUint16(14, day, true);
    cen.setUint32(16, crc, true);
    cen.setUint32(20, body.length, true);
    cen.setUint32(24, raw.length, true);
    cen.setUint16(28, nameBytes.length, true);
    cen.setUint32(42, this.offset, true);

    this.parts.push(new Uint8Array(local.buffer), nameBytes, body);
    this.central.push(new Uint8Array(cen.buffer), nameBytes);
    this.offset += 30 + nameBytes.length + body.length;
    return name;
  }

  get count() {
    return this.paths.size;
  }

  blob() {
    const size = this.central.reduce((n, p) => n + p.length, 0);
    const end = new DataView(new ArrayBuffer(22));
    end.setUint32(0, 0x06054b50, true);
    end.setUint16(8, this.count, true);
    end.setUint16(10, this.count, true);
    end.setUint32(12, size, true);
    end.setUint32(16, this.offset, true);
    return new Blob(this.parts.concat(this.central, [new Uint8Array(end.buffer)]), { type: 'application/zip' });
  }
}
