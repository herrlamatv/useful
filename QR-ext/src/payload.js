/* QR-ext - by herrlamatv
 * Vorlagen -> Text im QR-Code. Die Formate sind die, die Handy-Kameras
 * (iOS und Android) von sich aus erkennen.
 */

export const TYPES = ['text', 'wifi', 'email', 'tel', 'contact'];

export const EMPTY_FIELDS = {
  text: { text: '' },
  wifi: { ssid: '', password: '', security: 'WPA', hidden: false },
  email: { to: '', subject: '', body: '' },
  tel: { number: '' },
  contact: { first: '', last: '', org: '', phone: '', email: '', url: '' }
};

export function emptyFields(type) {
  return Object.assign({}, EMPTY_FIELDS[type] || EMPTY_FIELDS.text);
}

/** WLAN-Felder: \ ; , : " werden mit Backslash geschützt. */
export function escapeWifi(value) {
  return String(value).replace(/([\\;,:"])/g, '\\$1');
}

/** vCard-Felder: \ , ; und Zeilenumbrüche. */
export function escapeVcard(value) {
  return String(value)
    .replace(/\\/g, '\\\\')
    .replace(/([,;])/g, '\\$1')
    .replace(/\r?\n/g, '\\n');
}

/** Telefonnummer auf Ziffern und ein führendes + eindampfen. */
export function cleanPhone(value) {
  const s = String(value).trim();
  const digits = s.replace(/[^0-9]/g, '');
  return (s.startsWith('+') ? '+' : '') + digits;
}

export function wifiPayload({ ssid = '', password = '', security = 'WPA', hidden = false }) {
  if (!ssid) return '';
  const open = security === 'nopass';
  let out = 'WIFI:T:' + (open ? 'nopass' : security) + ';S:' + escapeWifi(ssid) + ';';
  if (!open && password) out += 'P:' + escapeWifi(password) + ';';
  if (hidden) out += 'H:true;';
  return out + ';';
}

export function emailPayload({ to = '', subject = '', body = '' }) {
  const addr = String(to).trim();
  if (!addr && !subject && !body) return '';
  const query = [];
  if (subject) query.push('subject=' + encodeURIComponent(subject));
  if (body) query.push('body=' + encodeURIComponent(body));
  return 'mailto:' + addr + (query.length ? '?' + query.join('&') : '');
}

export function telPayload({ number = '' }) {
  const n = cleanPhone(number);
  return n && n !== '+' ? 'tel:' + n : '';
}

export function contactPayload({ first = '', last = '', org = '', phone = '', email = '', url = '' }) {
  if (![first, last, org, phone, email, url].some((v) => String(v).trim())) return '';
  const lines = ['BEGIN:VCARD', 'VERSION:3.0'];
  lines.push('N:' + escapeVcard(last.trim()) + ';' + escapeVcard(first.trim()) + ';;;');
  const full = [first.trim(), last.trim()].filter(Boolean).join(' ') || org.trim();
  lines.push('FN:' + escapeVcard(full));
  if (org.trim()) lines.push('ORG:' + escapeVcard(org.trim()));
  if (cleanPhone(phone).replace('+', '')) lines.push('TEL;TYPE=CELL:' + cleanPhone(phone));
  if (email.trim()) lines.push('EMAIL:' + escapeVcard(email.trim()));
  if (url.trim()) lines.push('URL:' + escapeVcard(url.trim()));
  lines.push('END:VCARD');
  return lines.join('\r\n');
}

/** Felder einer Vorlage -> Text, der in den QR-Code kommt ('' = nichts da). */
export function buildPayload(type, fields = {}) {
  const f = Object.assign(emptyFields(type), fields);
  switch (type) {
    case 'wifi':
      return wifiPayload(f);
    case 'email':
      return emailPayload(f);
    case 'tel':
      return telPayload(f);
    case 'contact':
      return contactPayload(f);
    default:
      return String(f.text || '');
  }
}

/** Kurzer Text für den Verlauf. */
export function summary(type, fields = {}) {
  const f = Object.assign(emptyFields(type), fields);
  switch (type) {
    case 'wifi':
      return f.ssid;
    case 'email':
      return f.to || f.subject;
    case 'tel':
      return f.number;
    case 'contact':
      return [f.first, f.last].filter(Boolean).join(' ') || f.org || f.email || f.phone;
    default:
      return String(f.text).replace(/\s+/g, ' ').trim();
  }
}

/** Dateiname ohne Endung, z. B. "qr-example.com". */
export function fileSlug(type, fields = {}) {
  let base = summary(type, fields);
  if (type === 'text') {
    try {
      base = new URL(base).hostname || base;
    } catch (e) {}
  }
  const slug = String(base)
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9.]+/g, '-')
    .replace(/^[-.]+|[-.]+$/g, '')
    .slice(0, 40);
  return 'qr-' + (slug || type);
}
