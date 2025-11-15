<script lang="ts">
  export let backendStatus: 'online' | 'offline' | 'syncing' = 'offline';
  export let message: string = '';
</script>

<style>
.status { position: absolute; top: 60px; right: 20px; background: rgba(0,0,0,0.8); padding: 10px; border-radius: 8px; border: 1px solid rgba(127,255,255,0.25); z-index: 800; font-size: 11px; color: rgba(127,255,255,0.75); }
  .dot { display: inline-block; width: 12px; height: 12px; border-radius: 50%; margin-left: 6px; }
  .online { background: #00ff00; }
  .offline { background: #ff0000; }
  .syncing { background: #ffff00; }
  .msg { margin-left: 8px; font-size: 9px; }
</style>
<div class="status">
  Backend:
  <span class="dot {backendStatus}"></span>
  <span class="msg">{message}</span>
</div>