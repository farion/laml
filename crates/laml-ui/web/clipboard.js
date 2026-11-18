// Browser-only clipboard transport. Text and selection editing stay in Rust.
export async function copyText(text) {
  if (navigator.clipboard?.writeText) {
    try { await navigator.clipboard.writeText(text); return; } catch (_) {}
  }
  // HTTP hosts have no Async Clipboard API. Use a user-initiated copy command
  // with an isolated text field; never let eframe handle this helper's event.
  const previous = document.activeElement;
  const field = document.createElement('textarea');
  field.value = text;
  field.setAttribute('aria-hidden', 'true');
  field.style.cssText = 'position:fixed;left:-10000px;top:0;opacity:0';
  field.addEventListener('copy', event => {
    event.clipboardData.setData('text/plain', text);
    event.preventDefault();
    event.stopImmediatePropagation();
  });
  document.body.appendChild(field);
  try {
    field.focus(); field.select();
    if (!document.execCommand('copy')) throw new Error('Copy unavailable');
  } finally {
    field.remove();
    previous?.focus({preventScroll:true});
  }
}

export async function pasteText() {
  if (!navigator.clipboard?.readText) throw new Error('Paste unavailable');
  return await navigator.clipboard.readText();
}
