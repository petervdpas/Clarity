//#region src/lib/engineHost.ts
var e = class extends Error {}, t = [
	"clock",
	"random",
	"log",
	"language"
], n = ["clarity_alloc", "clarity_free"], r = ["clarity_tick", "clarity_draw"], i = 65536;
function a(e) {
	e.at = BigInt.asUintN(64, e.at + 11400714819323198485n);
	let t = e.at;
	return t = BigInt.asUintN(64, (t ^ t >> 30n) * 13787848793156543929n), t = BigInt.asUintN(64, (t ^ t >> 27n) * 10723151780598845931n), BigInt.asIntN(64, t ^ t >> 31n);
}
var o = class {
	#e;
	#t;
	#n;
	#r;
	#i;
	hears;
	#a = [];
	constructor(e, t, n, r, i) {
		this.#e = e, this.#t = t, this.#n = n, this.#r = r, this.#i = i, this.hears = {
			pointer: typeof t.clarity_pointer == "function",
			keys: typeof t.clarity_key == "function"
		};
	}
	call(t, n) {
		let r = (n) => new e(`engine ${this.#e} failed in ${t}: ${n}`), i = this.#t[t];
		if (typeof i != "function") throw r("it has no such function");
		this.#r.at = BigInt.asUintN(64, this.#i());
		let a;
		try {
			let e = new TextEncoder().encode(JSON.stringify(n)), t = this.#t.clarity_alloc(e.length);
			new Uint8Array(this.#t.memory.buffer, t, e.length).set(e);
			let r = BigInt.asUintN(64, i(t, e.length)), o = Number(r >> 32n), s = Number(r & 4294967295n), c = new TextDecoder().decode(new Uint8Array(this.#t.memory.buffer, o, s));
			this.#t.clarity_free(o, s), a = JSON.parse(c);
		} catch (t) {
			throw t instanceof e ? t : r(t instanceof Error ? t.message : String(t));
		}
		if (this.#t.memory.buffer.byteLength > this.#n * 1024 * 1024) throw r(`it grew past its memory limit of ${this.#n} MB`);
		if (a.error !== void 0) throw r(a.error);
		for (let e of Array.isArray(a.tells) ? a.tells : []) {
			if (typeof e?.tell != "string") throw r("it told something without a name");
			this.#a.push({
				tell: e.tell,
				values: Array.isArray(e.values) ? e.values : []
			});
		}
		return a.value;
	}
	frame(t, n = [], r = () => {}) {
		let i = (e, t) => (r(e), this.call(e, t));
		this.#a = [];
		for (let e of n) "hear" in e ? i(e.hear, e.values) : "pointer" in e ? this.hears.pointer && i("clarity_pointer", [
			e.pointer,
			e.x,
			e.y
		]) : this.hears.keys && i("clarity_key", [e.key, e.name]);
		i("clarity_tick", [t]);
		let a = i("clarity_draw", []);
		if (!Array.isArray(a)) throw new e(`engine ${this.#e} failed in clarity_draw: it gave no list of shapes`);
		let o = this.#a;
		return this.#a = [], {
			shapes: a,
			tells: o
		};
	}
}, s = (t) => (n) => new e(`engine ${t} ${n}`);
function c(e, t) {
	if (t.length < 4 || t[0] !== 0 || t[1] !== 97 || t[2] !== 115 || t[3] !== 109) throw s(e)("is no WebAssembly module");
}
function l(e, i, a) {
	let o = s(e);
	for (let e of WebAssembly.Module.imports(i)) if (e.module !== "clarity" || e.kind !== "function" || !t.includes(e.name) || !a.uses.includes(e.name)) throw o(`imports ${e.module}.${e.name}; an engine imports only the services it uses`);
	let c = WebAssembly.Module.exports(i), l = new Set(c.filter((e) => e.kind === "function").map((e) => e.name));
	for (let e of n) if (!l.has(e)) throw o(`lacks ${e} of the engine interface`);
	if (!c.some((e) => e.kind === "memory" && e.name === "memory")) throw o("lacks the memory of the engine interface");
	for (let e of a.screen ? r : []) if (!l.has(e)) throw o(`lacks ${e}, which an engine beside the screen needs`);
	for (let e of a.functions ?? []) if (!l.has(e)) throw o(`has no function ${e}`);
	if (a.opens && !l.has("clarity_open")) throw o("lacks clarity_open, which its on open needs");
}
function u(e, t, n, r) {
	let i = (e, t) => new Uint8Array(r().buffer, e, t);
	return {
		clock: () => BigInt(Math.trunc(t.clock())),
		random: () => a(n),
		log: (n, r) => t.log(`engine ${e}: ${new TextDecoder().decode(i(n, r))}`),
		language: (e, n) => {
			let r = new TextEncoder().encode(t.language());
			return i(e, Math.min(n, r.length)).set(r.subarray(0, n)), r.length;
		}
	};
}
function d(e, t, n, r, a, c) {
	let l = t.exports;
	if (c.memory = () => l.memory, l.memory.buffer.byteLength > n.megabytes * 1024 * 1024 + i) throw s(e)(`starts past its memory limit of ${n.megabytes} MB`);
	let u = new o(e, l, n.megabytes, r, a.seed);
	return n.opens && u.call("clarity_open", []), u;
}
var f = () => {
	throw Error("the engine is not made yet");
};
function p(e, t, n, r) {
	c(e, t);
	let i;
	try {
		i = new WebAssembly.Module(t);
	} catch (t) {
		throw s(e)(`is no WebAssembly module: ${t instanceof Error ? t.message : t}`);
	}
	l(e, i, n);
	let a = { at: 0n }, o = { memory: f }, p;
	try {
		p = new WebAssembly.Instance(i, { clarity: u(e, r, a, () => o.memory()) });
	} catch (t) {
		throw s(e)(`did not start: ${t instanceof Error ? t.message : t}`);
	}
	return d(e, p, n, a, r, o);
}
//#endregion
//#region src/lib/browserEngines.ts
var m = /* @__PURE__ */ new Map(), h = 0, g = "";
function _() {
	let e = crypto.getRandomValues(/* @__PURE__ */ new Uint32Array(2));
	return BigInt(e[0]) << 32n | BigInt(e[1]);
}
function v(e) {
	g = e;
}
function y(e, t, n) {
	let r = Uint8Array.from(atob(t), (e) => e.charCodeAt(0)), i = {
		clock: () => Date.now(),
		seed: _,
		log: (e) => console.info(e),
		language: () => g
	}, a = p(e, r, {
		...JSON.parse(n),
		screen: !0
	}, i);
	return m.set(++h, a), h;
}
function b(e) {
	return JSON.stringify(m.get(e)?.hears ?? {
		pointer: !1,
		keys: !1
	});
}
function x(e, t, n) {
	let r = m.get(e);
	if (!r) throw Error("the engine has stopped");
	return JSON.stringify(r.frame(t, JSON.parse(n)));
}
function S(e) {
	m.delete(e);
}
//#endregion
export { S as close, x as frame, b as hears, y as load, v as setLanguage };
