import sharp from 'sharp';
for (const [src,dst] of [
  ['../../docs/assets/og/how-to/configure-attested-routes.webp','/tmp/og-long.png'],
  ['../../docs/assets/og/start-here/integrator.webp','/tmp/og-short.png'],
  ['../../docs/assets/og/reference/cli-config-policy.webp','/tmp/og-mid.png'],
]) { await sharp(src).png().toFile(dst); }
console.log('converted 3 samples to PNG');
