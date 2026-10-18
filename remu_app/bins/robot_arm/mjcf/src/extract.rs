//! Kinematic extraction: pick the serial hinge chain out of the MJCF body
//! tree and reduce it to per-joint frames (offset / quat / axis / range).
//!
//! The chain convention emitted here: the frame of joint `i` is obtained from
//! the previous joint frame by `translate(offset) * rotate(quat) * rotate(axis, q_i)`,
//! so app-side forward kinematics is pure accumulation with no per-model
//! special cases.

use std::collections::HashMap;

use crate::parse::Element;

pub(crate) struct Model {
    pub model_name: Option<String>,
    pub joints: Vec<Joint>,
    pub obstacles: Vec<Obstacle>,
    pub tip_offset: V3,
    pub tip_quat: Quat,
    pub nu: usize,
    pub nq: usize,
    pub home_q: Option<Vec<f64>>,
}

pub(crate) struct Joint {
    pub name: String,
    pub offset: V3,
    pub quat: Quat,
    pub axis: V3,
    pub range: Option<[f64; 2]>,
    pub ctrl: Option<usize>,
    pub qpos_index: usize,
}

/// A capsule obstacle geom (`obstacle*`) in world coordinates.
pub(crate) struct Obstacle {
    pub a: V3,
    pub b: V3,
    pub radius: f64,
}

type V3 = [f64; 3];
/// (w, x, y, z)
type Quat = [f64; 4];

const IDENTITY: Quat = [1.0, 0.0, 0.0, 0.0];

pub(crate) fn extract(root: &Element) -> Result<Model, String> {
    let angle = angle_scale(root)?;
    let defaults = Defaults::collect(root);
    let (nu, ctrl) = actuators(root)?;

    let mut builder = Builder {
        defaults: &defaults,
        angle,
        qpos: 0,
    };
    let mut bodies = Vec::new();
    for worldbody in root.children_named("worldbody") {
        for body in worldbody.children_named("body") {
            bodies.push(builder.body(body, None)?);
        }
    }

    let base = pick_base(&bodies)?;
    let mut path = Vec::new();
    select_chain(base, &mut path)?;
    let (joints, tip) = assemble(&path, &ctrl)?;
    let nq = builder.qpos;

    Ok(Model {
        model_name: root.attr("model").map(str::to_owned),
        home_q: home(root, nq, &joints),
        obstacles: obstacles(root)?,
        joints,
        tip_offset: tip.0,
        tip_quat: tip.1,
        nu,
        nq,
    })
}

/// Capsule obstacles: worldbody-level geoms named `obstacle*` — the seam the
/// composed demo model (see the app's `assets/model_template.xml`) injects
/// them through.
fn obstacles(root: &Element) -> Result<Vec<Obstacle>, String> {
    let mut out = Vec::new();
    for worldbody in root.children_named("worldbody") {
        for geom in worldbody.children_named("geom") {
            let Some(name) = geom.attr("name") else {
                continue;
            };
            if !name.starts_with("obstacle") {
                continue;
            }
            if geom.attr("type") != Some("capsule") {
                return Err(format!(
                    "obstacle geom `{name}`: only capsules are supported"
                ));
            }
            let fromto = nums(
                geom.attr("fromto")
                    .ok_or_else(|| format!("obstacle geom `{name}`: missing fromto"))?,
            )?;
            let size = nums(
                geom.attr("size")
                    .ok_or_else(|| format!("obstacle geom `{name}`: missing size"))?,
            )?;
            if fromto.len() != 6 {
                return Err(format!("obstacle geom `{name}`: fromto needs 6 numbers"));
            }
            let radius = *size
                .first()
                .ok_or_else(|| format!("obstacle geom `{name}`: size needs a radius"))?;
            out.push(Obstacle {
                a: [fromto[0], fromto[1], fromto[2]],
                b: [fromto[3], fromto[4], fromto[5]],
                radius,
            });
        }
    }
    Ok(out)
}

fn angle_scale(root: &Element) -> Result<f64, String> {
    let angle = root
        .children_named("compiler")
        .find_map(|c| c.attr("angle"))
        .unwrap_or("degree");
    match angle {
        "radian" => Ok(1.0),
        "degree" => Ok(std::f64::consts::PI / 180.0),
        other => Err(format!(
            "<compiler angle={other:?}>: expected \"radian\" or \"degree\""
        )),
    }
}

// ── `<default>` classes ──

type TagDefaults = HashMap<String, HashMap<String, String>>;

struct Defaults {
    global: TagDefaults,
    classes: HashMap<String, TagDefaults>,
}

impl Defaults {
    fn collect(root: &Element) -> Self {
        let mut d = Self {
            global: TagDefaults::new(),
            classes: HashMap::new(),
        };
        for def in root.children_named("default") {
            d.walk(def, &TagDefaults::new());
        }
        d
    }

    fn walk(&mut self, el: &Element, inherited: &TagDefaults) {
        let mut own = inherited.clone();
        for child in &el.children {
            if child.tag == "default" {
                continue;
            }
            let entry = own.entry(child.tag.clone()).or_default();
            for (k, v) in &child.attrs {
                entry.insert(k.clone(), v.clone());
            }
        }
        match el.attr("class") {
            Some(class) => {
                self.classes.insert(class.to_owned(), own.clone());
            }
            None => {
                for (tag, attrs) in &own {
                    self.global
                        .entry(tag.clone())
                        .or_default()
                        .extend(attrs.clone());
                }
            }
        }
        for sub in el.children_named("default") {
            self.walk(sub, &own);
        }
    }

    fn joint_attrs(&self, el: &Element, class: Option<&str>) -> HashMap<String, String> {
        let mut attrs = match class {
            Some(c) => self.classes.get(c).and_then(|t| t.get("joint")).cloned(),
            None => self.global.get("joint").cloned(),
        }
        .unwrap_or_default();
        for (k, v) in &el.attrs {
            attrs.insert(k.clone(), v.clone());
        }
        attrs
    }
}

// ── body tree ──

struct Body {
    name: String,
    pos: V3,
    quat: Quat,
    joints: Vec<JointSpec>,
    free: bool,
    children: Vec<Body>,
}

struct JointSpec {
    name: String,
    kind: String,
    pos: V3,
    axis: V3,
    range: Option<[f64; 2]>,
    qpos_index: usize,
}

struct Builder<'a> {
    defaults: &'a Defaults,
    angle: f64,
    qpos: usize,
}

impl Builder<'_> {
    fn body(&mut self, el: &Element, class_ctx: Option<&str>) -> Result<Body, String> {
        let name = el.attr("name").unwrap_or("body").to_owned();
        for attr in ["euler", "axisangle", "zaxis", "xyaxes"] {
            if el.attr(attr).is_some() {
                return Err(format!(
                    "body `{name}`: `{attr}` orientation is not supported (use `quat`)"
                ));
            }
        }
        let class_ctx = el.attr("childclass").or(class_ctx);
        let pos = vec3(el.attr("pos"))?;
        let quat = quat(el.attr("quat"))?;
        let mut joints = Vec::new();
        let mut free = false;
        for child in &el.children {
            match child.tag.as_str() {
                "joint" => joints.push(self.joint(child, class_ctx)?),
                "freejoint" => {
                    free = true;
                    self.qpos += 7;
                }
                _ => {}
            }
        }
        let mut children = Vec::new();
        for child in el.children_named("body") {
            children.push(self.body(child, class_ctx)?);
        }
        Ok(Body {
            name,
            pos,
            quat,
            joints,
            free,
            children,
        })
    }

    fn joint(&mut self, el: &Element, class_ctx: Option<&str>) -> Result<JointSpec, String> {
        let attrs = self
            .defaults
            .joint_attrs(el, el.attr("class").or(class_ctx));
        let kind = attrs
            .get("type")
            .map(String::as_str)
            .unwrap_or("hinge")
            .to_owned();
        let name = attrs
            .get("name")
            .cloned()
            .unwrap_or_else(|| format!("joint{}", self.qpos));
        let pos = vec3(attrs.get("pos").map(String::as_str))?;
        let mut axis = vec3_or(attrs.get("axis").map(String::as_str), [0.0, 0.0, 1.0])?;
        normalize3(&mut axis).map_err(|()| format!("joint `{name}`: zero axis"))?;
        let range = match attrs.get("range") {
            Some(s) => {
                let v = nums(s)?;
                if v.len() != 2 {
                    return Err(format!("joint `{name}`: `range` needs 2 numbers"));
                }
                Some([v[0] * self.angle, v[1] * self.angle])
            }
            None => None,
        };
        let qpos_index = self.qpos;
        self.qpos += match kind.as_str() {
            "hinge" | "slide" => 1,
            "ball" => 4,
            "free" => 7,
            other => return Err(format!("joint `{name}`: unsupported type `{other}`")),
        };
        Ok(JointSpec {
            name,
            kind,
            pos,
            axis,
            range,
            qpos_index,
        })
    }
}

// ── chain selection ──

fn hinge_count(body: &Body) -> usize {
    let own = body.joints.iter().filter(|j| j.kind == "hinge").count();
    own + body.children.iter().map(hinge_count).sum::<usize>()
}

fn pick_base(bodies: &[Body]) -> Result<&Body, String> {
    let Some(base) = bodies.iter().max_by_key(|b| hinge_count(b)) else {
        return Err("no bodies under <worldbody>".to_owned());
    };
    let count = hinge_count(base);
    if count == 0 {
        return Err("no hinge-jointed body found".to_owned());
    }
    if bodies.iter().filter(|b| hinge_count(b) == count).count() > 1 {
        return Err("multiple hinge-jointed bodies; cannot pick the arm chain".to_owned());
    }
    Ok(base)
}

fn select_chain<'a>(body: &'a Body, path: &mut Vec<&'a Body>) -> Result<(), String> {
    path.push(body);
    let best = body
        .children
        .iter()
        .map(|c| (c, hinge_count(c)))
        .max_by_key(|(_, n)| *n);
    let Some((next, count)) = best else {
        return Ok(());
    };
    if count == 0 {
        return Ok(());
    }
    if body
        .children
        .iter()
        .filter(|c| hinge_count(c) == count)
        .count()
        > 1
    {
        return Err(format!(
            "body `{}`: ambiguous chain (multiple child bodies with hinge joints)",
            body.name
        ));
    }
    select_chain(next, path)
}

// ── frame assembly ──

fn assemble(
    path: &[&Body],
    ctrl: &HashMap<String, usize>,
) -> Result<(Vec<Joint>, (V3, Quat)), String> {
    let mut acc_pos = [0.0; 3];
    let mut acc_quat = IDENTITY;
    let mut joints = Vec::new();

    for body in path {
        acc_pos = add(acc_pos, rotate(acc_quat, body.pos));
        acc_quat = mul(acc_quat, body.quat);
        if body.joints.len() > 1 {
            return Err(format!(
                "body `{}`: multiple joints per body are not supported in the arm chain",
                body.name
            ));
        }
        for spec in &body.joints {
            if spec.kind != "hinge" {
                return Err(format!(
                    "joint `{}`: only hinge joints are supported in the arm chain",
                    spec.name
                ));
            }
            let anchor = add(acc_pos, rotate(acc_quat, spec.pos));
            joints.push(Joint {
                name: spec.name.clone(),
                offset: anchor,
                quat: acc_quat,
                axis: spec.axis,
                range: spec.range,
                ctrl: ctrl.get(&spec.name).copied(),
                qpos_index: spec.qpos_index,
            });
            // The next offsets live in this joint's frame: step back from the
            // anchor to the body origin (same orientation, `-joint pos`).
            acc_pos = neg(rotate(acc_quat, spec.pos));
            acc_quat = IDENTITY;
        }
    }
    if joints.is_empty() {
        return Err("no hinge joints in the chain".to_owned());
    }

    // Continue through fixed single-child bodies below the last joint (e.g.
    // the Panda `hand`) until a branch.
    let mut tip_pos = acc_pos;
    let mut tip_quat = acc_quat;
    let mut cur = *path.last().expect("path has at least one body");
    loop {
        let mut kids = cur.children.iter();
        let Some(next) = kids.next() else { break };
        if kids.next().is_some() || next.free || !next.joints.is_empty() {
            break;
        }
        tip_pos = add(tip_pos, rotate(tip_quat, next.pos));
        tip_quat = mul(tip_quat, next.quat);
        cur = next;
    }

    Ok((joints, (tip_pos, tip_quat)))
}

// ── actuators / keyframes ──

fn actuators(root: &Element) -> Result<(usize, HashMap<String, usize>), String> {
    let mut count = 0usize;
    let mut ctrl = HashMap::new();
    for act in root.children_named("actuator") {
        for a in &act.children {
            if let Some(joint) = a.attr("joint") {
                ctrl.entry(joint.to_owned()).or_insert(count);
            }
            count += 1;
        }
    }
    Ok((count, ctrl))
}

fn home(root: &Element, nq: usize, joints: &[Joint]) -> Option<Vec<f64>> {
    let key = root
        .children_named("keyframe")
        .flat_map(|kf| kf.children_named("key"))
        .find(|k| k.attr("name") == Some("home"))?;
    let qpos = nums(key.attr("qpos")?).ok()?;
    if qpos.len() != nq {
        return None;
    }
    Some(joints.iter().map(|j| qpos[j.qpos_index]).collect())
}

// ── value parsing / math ──

fn nums(s: &str) -> Result<Vec<f64>, String> {
    s.split_whitespace()
        .map(|t| t.parse::<f64>().map_err(|_| format!("bad number {t:?}")))
        .collect()
}

fn vec3(s: Option<&str>) -> Result<V3, String> {
    vec3_or(s, [0.0; 3])
}

fn vec3_or(s: Option<&str>, default: V3) -> Result<V3, String> {
    let Some(s) = s else {
        return Ok(default);
    };
    let v = nums(s)?;
    if v.len() != 3 {
        return Err(format!("expected 3 numbers, got {s:?}"));
    }
    Ok([v[0], v[1], v[2]])
}

fn quat(s: Option<&str>) -> Result<Quat, String> {
    let Some(s) = s else {
        return Ok(IDENTITY);
    };
    let v = nums(s)?;
    if v.len() != 4 {
        return Err(format!("expected 4 quaternion components, got {s:?}"));
    }
    let mut q = [v[0], v[1], v[2], v[3]];
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n == 0.0 {
        return Err(format!("zero quaternion {s:?}"));
    }
    for c in &mut q {
        *c /= n;
    }
    Ok(q)
}

fn normalize3(v: &mut V3) -> Result<(), ()> {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n == 0.0 {
        return Err(());
    }
    for c in v {
        *c /= n;
    }
    Ok(())
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn neg(a: V3) -> V3 {
    [-a[0], -a[1], -a[2]]
}

/// Hamilton product `a ∘ b`.
fn mul(a: Quat, b: Quat) -> Quat {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

fn rotate(q: Quat, v: V3) -> V3 {
    let [w, x, y, z] = q;
    let [vx, vy, vz] = v;
    let t = [
        2.0 * (y * vz - z * vy),
        2.0 * (z * vx - x * vz),
        2.0 * (x * vy - y * vx),
    ];
    [
        vx + w * t[0] + (y * t[2] - z * t[1]),
        vy + w * t[1] + (z * t[0] - x * t[2]),
        vz + w * t[2] + (x * t[1] - y * t[0]),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::parse;

    const MINI: &str = r#"
<mujoco model="mini">
  <compiler angle="degree"/>
  <default>
    <joint axis="0 0 1" damping="0.5"/>
    <default class="arm">
      <joint range="-90 90"/>
    </default>
    <default class="narrow">
      <joint range="-45 45"/>
    </default>
  </default>
  <worldbody>
    <body name="l0" childclass="arm">
      <joint name="j1"/>
      <body name="l1" pos="0.3 0 0" quat="0.70710678 0 0 0.70710678">
        <joint name="j2" class="narrow"/>
        <body name="l2" pos="0.4 0 0">
          <joint name="j3"/>
          <body name="tool" pos="0 0 0.107"/>
        </body>
      </body>
    </body>
  </worldbody>
  <actuator>
    <motor joint="j1"/>
    <motor joint="j2"/>
    <motor joint="j3"/>
  </actuator>
  <keyframe>
    <key name="home" qpos="0.1 0.2 0.3"/>
  </keyframe>
</mujoco>
"#;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn near3(a: V3, b: V3) -> bool {
        near(a[0], b[0]) && near(a[1], b[1]) && near(a[2], b[2])
    }

    /// Forward kinematics at `q = 0` from the emitted convention.
    fn fk_tip_at_zero(m: &Model) -> V3 {
        let mut pos = [0.0; 3];
        let mut q = IDENTITY;
        for j in &m.joints {
            pos = add(pos, rotate(q, j.offset));
            q = mul(q, j.quat);
        }
        add(pos, rotate(q, m.tip_offset))
    }

    #[test]
    fn extracts_chain_frames_defaults_and_home() {
        let root = parse::load_str(MINI).unwrap();
        let m = extract(&root).unwrap();

        assert_eq!((m.nu, m.nq), (3, 3));
        assert_eq!(m.joints.len(), 3);

        let [j1, j2, j3] = &m.joints[..] else {
            panic!()
        };
        // Default-class inheritance, degrees converted to radians.
        assert!(near(j1.range.unwrap()[0], -std::f64::consts::FRAC_PI_2));
        assert!(near(j1.range.unwrap()[1], std::f64::consts::FRAC_PI_2));
        assert!(near(j2.range.unwrap()[0], -std::f64::consts::FRAC_PI_4));
        assert!(near(j3.range.unwrap()[1], std::f64::consts::FRAC_PI_2));
        assert!(near3(j1.axis, [0.0, 0.0, 1.0]));

        // Frame accumulation: offsets live in the previous joint frame, the
        // body rotation is carried by `quat` (rotated into the FK below).
        assert!(near3(j2.offset, [0.3, 0.0, 0.0]));
        assert!(near(j2.quat[0], std::f64::consts::FRAC_1_SQRT_2));
        assert!(near(j2.quat[3], std::f64::consts::FRAC_1_SQRT_2));
        assert!(near3(j3.offset, [0.4, 0.0, 0.0]), "{:?}", j3.offset);
        assert!(near3(m.tip_offset, [0.0, 0.0, 0.107]));
        assert!(
            near3(fk_tip_at_zero(&m), [0.3, 0.4, 0.107]),
            "{:?}",
            fk_tip_at_zero(&m)
        );

        assert_eq!(j1.ctrl, Some(0));
        assert_eq!(j3.ctrl, Some(2));
        assert_eq!(m.home_q.as_deref(), Some(&[0.1, 0.2, 0.3][..]));
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mjcf_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn include_files_are_spliced_and_tracked() {
        let dir = tempdir("include");
        std::fs::write(
            dir.join("outer.xml"),
            "<mujoco model=\"outer\"><include file=\"inner.xml\"/></mujoco>",
        )
        .unwrap();
        std::fs::write(
            dir.join("inner.xml"),
            "<mujoco model=\"inner\">\
             <compiler angle=\"radian\"/>\
             <worldbody>\
             <body name=\"l0\"><joint name=\"j1\" range=\"-1 1\"/>\
             <body name=\"l1\" pos=\"0.5 0 0\"><joint name=\"j2\" range=\"-2 2\"/></body>\
             </body>\
             </worldbody>\
             </mujoco>",
        )
        .unwrap();

        let loaded = parse::load_file(&dir.join("outer.xml")).unwrap();
        assert_eq!(loaded.files.len(), 2);
        let m = extract(&loaded.root).unwrap();
        assert_eq!(m.joints.len(), 2);
        assert!(near(m.joints[1].offset[0], 0.5));
        assert!(near(m.joints[1].range.unwrap()[1], 2.0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[ignore = "needs the menagerie checkout (MUJOCO_MENAGERIE_DIR)"]
    #[allow(
        clippy::approx_constant,
        reason = "the qpos value comes from the model"
    )]
    fn panda_chain_from_menagerie() {
        let dir = std::env::var("MUJOCO_MENAGERIE_DIR").expect("MUJOCO_MENAGERIE_DIR");
        let path = std::path::Path::new(&dir).join("franka_emika_panda/scene.xml");
        let loaded = parse::load_file(&path).expect("load panda scene");
        let m = extract(&loaded.root).expect("extract panda chain");

        assert_eq!((m.nu, m.nq, m.joints.len()), (8, 9, 7));
        assert_eq!(m.joints[0].name, "joint1");
        assert_eq!(m.joints[0].ctrl, Some(0));
        assert!(near(m.joints[0].range.unwrap()[0], -2.8973));
        assert!(near3(m.joints[0].offset, [0.0, 0.0, 0.333]));
        assert!(
            near3(m.joints[1].offset, [0.0, 0.0, 0.0]),
            "joint2 shares joint1's anchor"
        );
        assert!(near(m.joints[1].range.unwrap()[0], -1.7628));
        assert!(
            near3(m.joints[2].offset, [0.0, -0.316, 0.0]),
            "{:?}",
            m.joints[2].offset
        );
        assert!(
            near3(m.joints[3].offset, [0.0825, 0.0, 0.0]),
            "{:?}",
            m.joints[3].offset
        );
        assert!(near3(m.tip_offset, [0.0, 0.0, 0.107]), "{:?}", m.tip_offset);
        let home = m.home_q.expect("home keyframe");
        assert!(near(home[3], -1.57079), "{home:?}");
    }
}
