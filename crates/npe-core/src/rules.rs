use crate::{Outcome, PlayerId};
use rhai::serde::{from_dynamic, to_dynamic};
use rhai::{Array, CallFnOptions, Dynamic, Engine, Map, Scope, AST, INT};
use serde_json::Value;

/// Compiled rules script plus a locked-down Rhai engine.
///
/// Hooks: `init(ctx)`, `active(s)`, `observe(s, p)`, `validate(s, p, a)`,
/// `default_action(s, p)`, `eliminate(s, p)`, `apply(s, actions)`,
/// `outcome(s, forced)`, `view(s)`.
pub struct Rules {
    engine: Engine,
    ast: AST,
}

const HOOKS: &[(&str, usize)] = &[
    ("init", 1),
    ("active", 1),
    ("observe", 2),
    ("validate", 3),
    ("default_action", 2),
    ("eliminate", 2),
    ("apply", 2),
    ("outcome", 2),
    ("view", 1),
];

fn splitmix(x: INT) -> INT {
    let mut z = (x as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    // 53 bits, so the value also survives a trip through JavaScript numbers.
    ((z ^ (z >> 31)) >> 11) as INT
}

fn build_engine() -> Engine {
    let mut e = Engine::new();
    e.set_max_operations(5_000_000);
    e.set_max_call_levels(64);
    e.set_max_expr_depths(128, 64);
    e.set_max_string_size(64 * 1024);
    e.set_max_array_size(100_000);
    e.set_max_map_size(10_000);
    e.disable_symbol("eval");
    e.on_print(|_| {});
    e.on_debug(|_, _, _| {});
    e.register_fn("rng_next", splitmix);
    // Platform libm differs between targets, so route every transcendental
    // function through the pure-Rust `libm`. `sin`/`cos` take radians.
    e.register_fn("sin", libm::sin);
    e.register_fn("cos", libm::cos);
    e.register_fn("tan", libm::tan);
    e.register_fn("asin", libm::asin);
    e.register_fn("acos", libm::acos);
    e.register_fn("atan", libm::atan);
    e.register_fn("atan", libm::atan2);
    e.register_fn("atan2", libm::atan2);
    e.register_fn("hypot", libm::hypot);
    e.register_fn("exp", libm::exp);
    e.register_fn("ln", libm::log);
    e.register_fn("sin_deg", |d: f64| libm::sin(d * core::f64::consts::PI / 180.0));
    e.register_fn("cos_deg", |d: f64| libm::cos(d * core::f64::consts::PI / 180.0));
    e.register_fn("atan2_deg", |y: f64, x: f64| {
        libm::atan2(y, x) * 180.0 / core::f64::consts::PI
    });
    // Keeps replays small. Two decimals are enough for drawing.
    e.register_fn("round2", |x: f64| libm::round(x * 100.0) / 100.0);
    e.register_fn("is_num", |v: Dynamic| {
        v.is_int() || v.as_float().map(f64::is_finite).unwrap_or(false)
    });
    e.register_fn("num", |v: Dynamic| -> f64 {
        v.as_int().map(|i| i as f64).or_else(|_| v.as_float()).unwrap_or(0.0)
    });
    e
}

impl Rules {
    pub fn compile(script: &str) -> Result<Self, String> {
        let engine = build_engine();
        let ast = engine.compile(script).map_err(|e| format!("rules: {e}"))?;
        for (name, arity) in HOOKS {
            if !ast.iter_functions().any(|f| f.name == *name && f.params.len() == *arity) {
                return Err(format!("rules: missing hook {name} with {arity} parameter(s)"));
            }
        }
        Ok(Rules { engine, ast })
    }

    fn call(&self, name: &str, args: Vec<Dynamic>) -> Result<Dynamic, String> {
        let mut scope = Scope::new();
        self.engine
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut scope,
                &self.ast,
                name,
                args,
            )
            .map_err(|e| format!("rules.{name}: {e}"))
    }

    pub fn init(&self, players: usize, config: &Value, seed: u32) -> Result<Dynamic, String> {
        let mut ctx = Map::new();
        ctx.insert("players".into(), Dynamic::from(players as INT));
        ctx.insert("seed".into(), Dynamic::from(seed as INT));
        ctx.insert("config".into(), dynamic(config)?);
        self.call("init", vec![ctx.into()])
    }

    pub fn active(&self, s: &Dynamic) -> Result<Vec<PlayerId>, String> {
        value(&self.call("active", vec![s.clone()])?)
    }

    pub fn observe(&self, s: &Dynamic, p: PlayerId) -> Result<Value, String> {
        value(&self.call("observe", vec![s.clone(), (p as INT).into()])?)
    }

    /// The script returns "" for a legal action and a message otherwise.
    /// A script error here means the action had an unexpected shape.
    pub fn validate(&self, s: &Dynamic, p: PlayerId, a: &Value) -> Result<(), String> {
        let d = dynamic(a)?;
        let r = self.call("validate", vec![s.clone(), (p as INT).into(), d])?;
        match r.into_string() {
            Ok(m) if m.is_empty() => Ok(()),
            Ok(m) => Err(m),
            Err(t) => Err(format!("validate returned {t}")),
        }
    }

    pub fn default_action(&self, s: &Dynamic, p: PlayerId) -> Result<Value, String> {
        value(&self.call("default_action", vec![s.clone(), (p as INT).into()])?)
    }

    pub fn eliminate(&self, s: Dynamic, p: PlayerId) -> Result<Dynamic, String> {
        self.call("eliminate", vec![s, (p as INT).into()])
    }

    /// `actions[p]` is `()` for players that do not act this step.
    pub fn apply(&self, s: Dynamic, actions: &[Option<Value>]) -> Result<Dynamic, String> {
        let mut arr = Array::with_capacity(actions.len());
        for a in actions {
            arr.push(match a {
                Some(v) => dynamic(v)?,
                None => Dynamic::UNIT,
            });
        }
        self.call("apply", vec![s, arr.into()])
    }

    pub fn outcome(&self, s: &Dynamic, forced: bool, n: usize) -> Result<Option<Outcome>, String> {
        let r = self.call("outcome", vec![s.clone(), forced.into()])?;
        if r.is_unit() {
            return Ok(None);
        }
        let o: Outcome = value(&r)?;
        if o.ranks.len() != n || o.scores.len() != n {
            return Err("rules.outcome: ranks and scores need one entry per player".into());
        }
        Ok(Some(o))
    }

    pub fn view(&self, s: &Dynamic) -> Result<Value, String> {
        value(&self.call("view", vec![s.clone()])?)
    }
}

fn dynamic(v: &Value) -> Result<Dynamic, String> {
    to_dynamic(v).map_err(|e| e.to_string())
}

fn value<T: serde::de::DeserializeOwned>(d: &Dynamic) -> Result<T, String> {
    from_dynamic(d).map_err(|e| format!("rules: bad return value: {e}"))
}
