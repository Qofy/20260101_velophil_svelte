<script lang="ts">
  export let visible: boolean = false;
  export let getPositions: () => Float32Array;
  export let zoom: number = 1.0;
  export let translation: { x: number; y: number; z: number } = { x: 0, y: 0, z: 0 };
  export let rotation: { x: number; y: number; z: number } = { x: 0, y: 0, z: 0 };
  export let perspectivePx: number = 1000.0;
  export const pointSize: number = 3.0;

  let canvas: HTMLCanvasElement | null = null;
  let gl: WebGL2RenderingContext | null = null;
  let prog: WebGLProgram | null = null;
  let quadBuf: WebGLBuffer | null = null;
  let instBuf: WebGLBuffer | null = null;
  let uRes: WebGLUniformLocation | null = null;
  let uZoom: WebGLUniformLocation | null = null;
  let uTrans: WebGLUniformLocation | null = null;
  let uRot: WebGLUniformLocation | null = null;
  let uPersp: WebGLUniformLocation | null = null;
  let aPosLoc = 0;
  let raf = 0;

  function createShader(gl: WebGL2RenderingContext, type: number, src: string) {
    const sh = gl.createShader(type)!;
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      console.error('shader error', gl.getShaderInfoLog(sh));
    }
    return sh;
  }
  function createProgram(gl: WebGL2RenderingContext, vsSrc: string, fsSrc: string) {
    const vs = createShader(gl, gl.VERTEX_SHADER, vsSrc);
    const fs = createShader(gl, gl.FRAGMENT_SHADER, fsSrc);
    const p = gl.createProgram()!;
    gl.attachShader(p, vs);
    gl.attachShader(p, fs);
    gl.bindAttribLocation(p, 0, 'a_pos');
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) {
      console.error('link error', gl.getProgramInfoLog(p));
    }
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    return p;
  }

  function initGL() {
    if (!canvas) return;
    gl = canvas.getContext('webgl2', { antialias: true, alpha: true }) as WebGL2RenderingContext | null;
    if (!gl) return;
    const vs = `#version 300 es\nprecision highp float;\nlayout(location=0) in vec3 a_pos;\nuniform vec2 u_res;\nuniform vec3 u_trans;\nuniform vec3 u_rot;\nuniform float u_zoom;\nuniform float u_persp;\n\nvec3 rotateX(vec3 p, float a){ float c=cos(a), s=sin(a); return vec3(p.x, c*p.y - s*p.z, s*p.y + c*p.z); }\nvec3 rotateY(vec3 p, float a){ float c=cos(a), s=sin(a); return vec3(c*p.x + s*p.z, p.y, -s*p.x + c*p.z); }\nvec3 rotateZ(vec3 p, float a){ float c=cos(a), s=sin(a); return vec3(c*p.x - s*p.y, s*p.x + c*p.y, p.z); }\n\nvoid main(){\n  // world -> rotate -> translate\n  vec3 p = a_pos;\n  p = rotateX(p, u_rot.x);\n  p = rotateY(p, u_rot.y);\n  p = rotateZ(p, u_rot.z);\n  p += u_trans;\n  // simple perspective approximation\n  float denom = max(0.01, (u_persp - p.z));\n  float sx = (p.x * u_persp / denom) * u_zoom + 0.5 * u_res.x;\n  float sy = (p.y * u_persp / denom) * u_zoom + 0.5 * u_res.y;\n  float cx = (sx / (0.5 * u_res.x)) - 1.0;\n  float cy = 1.0 - (sy / (0.5 * u_res.y));\n  gl_Position = vec4(cx, cy, 0.0, 1.0);\n  float ps = ${'${pointSize.toFixed(1)}'} * (u_persp / denom);\n  gl_PointSize = clamp(ps, 1.0, 20.0);\n}`;
    const fs = `#version 300 es\nprecision mediump float;\nout vec4 frag;\nvoid main(){\n  // Use gl_FragCoord not ideal; instead assume quad coords produce smooth glow based on distance from center\n  // Approximate radial glow in square by using min distance to corners via a diamond shape\n  // For a simple glow, just use alpha falloff from center based on normalized coords passed via geometry (omitted), so fallback to soft edges\n  frag = vec4(0.0, 1.0, 1.0, 0.75);\n}`;
    prog = createProgram(gl, vs, fs);
    uRes = gl.getUniformLocation(prog!, 'u_res');
    uZoom = gl.getUniformLocation(prog!, 'u_zoom');
    uTrans = gl.getUniformLocation(prog!, 'u_trans');
    uRot = gl.getUniformLocation(prog!, 'u_rot');
    uPersp = gl.getUniformLocation(prog!, 'u_persp');
    // Quad buffer (two triangles forming a square centered at 0)
    const quad = new Float32Array([
      -0.5, -0.5,
       0.5, -0.5,
       0.5,  0.5,
      -0.5, -0.5,
       0.5,  0.5,
      -0.5,  0.5,
    ]);
    quadBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, quadBuf);
    gl.bufferData(gl.ARRAY_BUFFER, quad, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    // Instance buffer for world positions
    instBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 0, 0);
    gl.vertexAttribDivisor(1, 1);
    gl.clearColor(0,0,0,0);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  }

  function resize() {
    if (!canvas || !gl) return;
    const dpr = window.devicePixelRatio || 1;
    const w = Math.floor(canvas.clientWidth * dpr);
    const h = Math.floor(canvas.clientHeight * dpr);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w; canvas.height = h;
      gl.viewport(0,0,w,h);
    }
  }

  function frame() {
    raf = 0;
    if (!visible || !canvas || !gl || !prog || !instBuf || !quadBuf) return;
    resize();
    gl.useProgram(prog);
    gl.uniform2f(uRes, canvas.width, canvas.height);
    gl.uniform1f(uZoom, zoom);
    gl.uniform3f(uTrans, translation.x, translation.y, translation.z);
    gl.uniform3f(uRot, rotation.x, rotation.y, rotation.z);
    gl.uniform1f(uPersp, perspectivePx);
    const pos = (getPositions && getPositions()) || new Float32Array();
    // pack positions as vec3 world coords
    const count = Math.floor(pos.length / 3);
    const tmp = new Float32Array(count * 3);
    for (let i=0;i<count;i++) { tmp[i*3] = pos[i*3]; tmp[i*3+1] = pos[i*3+1]; tmp[i*3+2] = pos[i*3+2]; }
    gl.bindBuffer(gl.ARRAY_BUFFER, instBuf);
    gl.bufferData(gl.ARRAY_BUFFER, tmp, gl.DYNAMIC_DRAW);
    gl.clear(gl.COLOR_BUFFER_BIT);
    // Bind quad buffer for location 0
    gl.bindBuffer(gl.ARRAY_BUFFER, quadBuf);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, count);
    raf = requestAnimationFrame(frame);
  }

  $: if (visible) {
    if (!gl) initGL();
    if (!raf) raf = requestAnimationFrame(frame);
  } else {
    if (raf) { cancelAnimationFrame(raf); raf = 0; }
  }

  function onResize(){ if (visible) resize(); }
  function onMount(){ initGL(); resize(); if (visible && !raf) raf = requestAnimationFrame(frame); }
  function onDestroy(){ if (raf) cancelAnimationFrame(raf); }
</script>

<style>
  .layer {
    position: absolute;
    inset: 0;
    z-index: 100; /* behind menus but above background */
    pointer-events: none;
  }
  canvas { width: 100%; height: 100%; display: block; }
</style>

{#if visible}
  <div class="layer">
    <canvas bind:this={canvas} on:resize={onResize} on:introstart={onMount} on:outrostart={onDestroy}></canvas>
  </div>
{/if}
