import sharp from "file://node_modules/.pnpm/sharp@0.34.5/node_modules/sharp/lib/index.js";
const svg = `<svg width="1024" height="1024" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f43f5e"/><stop offset="1" stop-color="#a855f7"/></linearGradient></defs><rect width="1024" height="1024" rx="224" fill="url(#g)"/><path d="M408 322 L408 702 L726 512 Z" fill="#ffffff"/></svg>`;
await sharp(Buffer.from(svg)).png().toFile("app-icon.png");
console.log("wrote app-icon.png");
