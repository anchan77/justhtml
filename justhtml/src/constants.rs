//! HTML5 spec constants for tree building.
//!
//! Corresponds to Python's `constants.py`.

use phf::phf_map;
use phf::phf_set;

/// Foreign attribute adjustments for SVG/MathML.
/// Maps lowercase attribute names to (prefix, local_name, namespace_url).
pub static FOREIGN_ATTRIBUTE_ADJUSTMENTS: phf::Map<&'static str, (&'static str, &'static str, &'static str)> = phf_map! {
    "xlink:actuate" => ("xlink", "actuate", "http://www.w3.org/1999/xlink"),
    "xlink:arcrole" => ("xlink", "arcrole", "http://www.w3.org/1999/xlink"),
    "xlink:href" => ("xlink", "href", "http://www.w3.org/1999/xlink"),
    "xlink:role" => ("xlink", "role", "http://www.w3.org/1999/xlink"),
    "xlink:show" => ("xlink", "show", "http://www.w3.org/1999/xlink"),
    "xlink:title" => ("xlink", "title", "http://www.w3.org/1999/xlink"),
    "xlink:type" => ("xlink", "type", "http://www.w3.org/1999/xlink"),
    "xml:lang" => ("xml", "lang", "http://www.w3.org/XML/1998/namespace"),
    "xml:space" => ("xml", "space", "http://www.w3.org/XML/1998/namespace"),
    "xmlns" => ("", "xmlns", "http://www.w3.org/2000/xmlns/"),
    "xmlns:xlink" => ("xmlns", "xlink", "http://www.w3.org/2000/xmlns/"),
};

/// MathML attribute case adjustments.
pub static MATHML_ATTRIBUTE_ADJUSTMENTS: phf::Map<&'static str, &'static str> = phf_map! {
    "definitionurl" => "definitionURL",
};

/// SVG attribute case adjustments.
pub static SVG_ATTRIBUTE_ADJUSTMENTS: phf::Map<&'static str, &'static str> = phf_map! {
    "attributename" => "attributeName",
    "attributetype" => "attributeType",
    "basefrequency" => "baseFrequency",
    "baseprofile" => "baseProfile",
    "calcmode" => "calcMode",
    "clippathunits" => "clipPathUnits",
    "diffuseconstant" => "diffuseConstant",
    "edgemode" => "edgeMode",
    "filterunits" => "filterUnits",
    "glyphref" => "glyphRef",
    "gradienttransform" => "gradientTransform",
    "gradientunits" => "gradientUnits",
    "kernelmatrix" => "kernelMatrix",
    "kernelunitlength" => "kernelUnitLength",
    "keypoints" => "keyPoints",
    "keysplines" => "keySplines",
    "keytimes" => "keyTimes",
    "lengthadjust" => "lengthAdjust",
    "limitingconeangle" => "limitingConeAngle",
    "markerheight" => "markerHeight",
    "markerunits" => "markerUnits",
    "markerwidth" => "markerWidth",
    "maskcontentunits" => "maskContentUnits",
    "maskunits" => "maskUnits",
    "numoctaves" => "numOctaves",
    "pathlength" => "pathLength",
    "patterncontentunits" => "patternContentUnits",
    "patterntransform" => "patternTransform",
    "patternunits" => "patternUnits",
    "pointsatx" => "pointsAtX",
    "pointsaty" => "pointsAtY",
    "pointsatz" => "pointsAtZ",
    "preservealpha" => "preserveAlpha",
    "preserveaspectratio" => "preserveAspectRatio",
    "primitiveunits" => "primitiveUnits",
    "refx" => "refX",
    "refy" => "refY",
    "repeatcount" => "repeatCount",
    "repeatdur" => "repeatDur",
    "requiredextensions" => "requiredExtensions",
    "requiredfeatures" => "requiredFeatures",
    "specularconstant" => "specularConstant",
    "specularexponent" => "specularExponent",
    "spreadmethod" => "spreadMethod",
    "startoffset" => "startOffset",
    "stddeviation" => "stdDeviation",
    "stitchtiles" => "stitchTiles",
    "surfacescale" => "surfaceScale",
    "systemlanguage" => "systemLanguage",
    "tablevalues" => "tableValues",
    "targetx" => "targetX",
    "targety" => "targetY",
    "textlength" => "textLength",
    "viewbox" => "viewBox",
    "viewtarget" => "viewTarget",
    "xchannelselector" => "xChannelSelector",
    "ychannelselector" => "yChannelSelector",
    "zoomandpan" => "zoomAndPan",
};

/// SVG tag name case adjustments.
pub static SVG_TAG_NAME_ADJUSTMENTS: phf::Map<&'static str, &'static str> = phf_map! {
    "altglyph" => "altGlyph",
    "altglyphdef" => "altGlyphDef",
    "altglyphitem" => "altGlyphItem",
    "animatecolor" => "animateColor",
    "animatemotion" => "animateMotion",
    "animatetransform" => "animateTransform",
    "clippath" => "clipPath",
    "feblend" => "feBlend",
    "fecolormatrix" => "feColorMatrix",
    "fecomponenttransfer" => "feComponentTransfer",
    "fecomposite" => "feComposite",
    "feconvolvematrix" => "feConvolveMatrix",
    "fediffuselighting" => "feDiffuseLighting",
    "fedisplacementmap" => "feDisplacementMap",
    "fedistantlight" => "feDistantLight",
    "feflood" => "feFlood",
    "fefunca" => "feFuncA",
    "fefuncb" => "feFuncB",
    "fefuncg" => "feFuncG",
    "fefuncr" => "feFuncR",
    "fegaussianblur" => "feGaussianBlur",
    "feimage" => "feImage",
    "femerge" => "feMerge",
    "femergenode" => "feMergeNode",
    "femorphology" => "feMorphology",
    "feoffset" => "feOffset",
    "fepointlight" => "fePointLight",
    "fespecularlighting" => "feSpecularLighting",
    "fespotlight" => "feSpotLight",
    "fetile" => "feTile",
    "feturbulence" => "feTurbulence",
    "foreignobject" => "foreignObject",
    "glyphref" => "glyphRef",
    "lineargradient" => "linearGradient",
    "radialgradient" => "radialGradient",
    "textpath" => "textPath",
};

/// Namespace URL to prefix mapping.
pub static NAMESPACE_URL_TO_PREFIX: phf::Map<&'static str, &'static str> = phf_map! {
    "http://www.w3.org/1999/xhtml" => "html",
    "http://www.w3.org/1998/Math/MathML" => "math",
    "http://www.w3.org/2000/svg" => "svg",
};

/// HTML namespace URL.
pub const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
/// MathML namespace URL.
pub const MATHML_NAMESPACE: &str = "http://www.w3.org/1998/Math/MathML";
/// SVG namespace URL.
pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// HTML integration points (SVG/MathML elements that allow HTML content).
/// Stored as (namespace_prefix, element_name) tuples.
pub static HTML_INTEGRATION_POINT_SET: phf::Set<&'static str> = phf_set! {
    "math:annotation-xml",
    "svg:foreignObject",
    "svg:desc",
    "svg:title",
};

/// MathML text integration points.
pub static MATHML_TEXT_INTEGRATION_POINT_SET: phf::Set<&'static str> = phf_set! {
    "math:mi",
    "math:mo",
    "math:mn",
    "math:ms",
    "math:mtext",
};

/// Check if (namespace_prefix, name) is an HTML integration point.
pub fn is_html_integration_point(prefix: &str, name: &str) -> bool {
    let key = format!("{}:{}", prefix, name);
    HTML_INTEGRATION_POINT_SET.contains(key.as_str())
}

/// Check if (namespace_prefix, name) is a MathML text integration point.
pub fn is_mathml_text_integration_point(prefix: &str, name: &str) -> bool {
    let key = format!("{}:{}", prefix, name);
    MATHML_TEXT_INTEGRATION_POINT_SET.contains(key.as_str())
}

/// Heading elements.
pub static HEADING_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "h1", "h2", "h3", "h4", "h5", "h6",
};

/// Formatting elements.
pub static FORMATTING_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "a", "b", "big", "code", "em", "font", "i", "nobr", "s",
    "small", "strike", "strong", "tt", "u",
};

/// Special elements.
pub static SPECIAL_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "address", "applet", "area", "article", "aside", "base", "basefont",
    "bgsound", "blockquote", "body", "br", "button", "caption", "center",
    "col", "colgroup", "dd", "details", "dialog", "dir", "div", "dl",
    "dt", "embed", "fieldset", "figcaption", "figure", "footer", "form",
    "frame", "frameset", "h1", "h2", "h3", "h4", "h5", "h6", "head",
    "header", "hgroup", "hr", "html", "iframe", "img", "input", "keygen",
    "li", "link", "listing", "main", "marquee", "menu", "menuitem",
    "meta", "nav", "noembed", "noframes", "noscript", "object", "ol",
    "p", "param", "plaintext", "pre", "script", "search", "section",
    "select", "source", "style", "summary", "table", "tbody", "td",
    "template", "textarea", "tfoot", "th", "thead", "title", "tr",
    "track", "ul", "wbr",
};

/// Default scope terminators.
pub static DEFAULT_SCOPE_TERMINATORS: phf::Set<&'static str> = phf_set! {
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
};

/// Button scope terminators (default + button).
pub static BUTTON_SCOPE_TERMINATORS: phf::Set<&'static str> = phf_set! {
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "button",
};

/// List item scope terminators (default + ol, ul).
pub static LIST_ITEM_SCOPE_TERMINATORS: phf::Set<&'static str> = phf_set! {
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "ol", "ul",
};

/// Definition scope terminators (default + dl).
pub static DEFINITION_SCOPE_TERMINATORS: phf::Set<&'static str> = phf_set! {
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "dl",
};

/// Table scope terminators.
pub static TABLE_SCOPE_TERMINATORS: phf::Set<&'static str> = phf_set! {
    "html", "table", "template",
};

/// Table foster parenting targets.
pub static TABLE_FOSTER_TARGETS: phf::Set<&'static str> = phf_set! {
    "table", "tbody", "tfoot", "thead", "tr",
};

/// Table allowed children.
pub static TABLE_ALLOWED_CHILDREN: phf::Set<&'static str> = phf_set! {
    "caption", "colgroup", "tbody", "tfoot", "thead", "tr", "td", "th",
    "script", "template", "style",
};

/// Implied end tags.
pub static IMPLIED_END_TAGS: phf::Set<&'static str> = phf_set! {
    "dd", "dt", "li", "option", "optgroup", "p", "rb", "rp", "rt", "rtc",
};

/// Void elements (self-closing, no content).
pub static VOID_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr",
};

/// Foreign content breakout elements.
pub static FOREIGN_BREAKOUT_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "b", "big", "blockquote", "body", "br", "center", "code", "dd",
    "div", "dl", "dt", "em", "embed", "h1", "h2", "h3", "h4", "h5",
    "h6", "head", "hr", "i", "img", "li", "listing", "menu", "meta",
    "nobr", "ol", "p", "pre", "ruby", "s", "small", "span", "strong",
    "strike", "sub", "sup", "table", "tt", "u", "ul", "var",
};

/// Quirky public ID prefixes for DOCTYPE.
pub static QUIRKY_PUBLIC_PREFIXES: &[&str] = &[
    "-//advasoft ltd//dtd html 3.0 aswedit + extensions//",
    "-//as//dtd html 3.0 aswedit + extensions//",
    "-//ietf//dtd html 2.0 level 1//",
    "-//ietf//dtd html 2.0 level 2//",
    "-//ietf//dtd html 2.0 strict level 1//",
    "-//ietf//dtd html 2.0 strict level 2//",
    "-//ietf//dtd html 2.0 strict//",
    "-//ietf//dtd html 2.0//",
    "-//ietf//dtd html 2.1e//",
    "-//ietf//dtd html 3.0//",
    "-//ietf//dtd html 3.2 final//",
    "-//ietf//dtd html 3.2//",
    "-//ietf//dtd html 3//",
    "-//ietf//dtd html level 0//",
    "-//ietf//dtd html level 1//",
    "-//ietf//dtd html level 2//",
    "-//ietf//dtd html level 3//",
    "-//ietf//dtd html strict level 0//",
    "-//ietf//dtd html strict level 1//",
    "-//ietf//dtd html strict level 2//",
    "-//ietf//dtd html strict level 3//",
    "-//ietf//dtd html strict//",
    "-//ietf//dtd html//",
    "-//metrius//dtd metrius presentational//",
    "-//microsoft//dtd internet explorer 2.0 html strict//",
    "-//microsoft//dtd internet explorer 2.0 html//",
    "-//microsoft//dtd internet explorer 2.0 tables//",
    "-//microsoft//dtd internet explorer 3.0 html strict//",
    "-//microsoft//dtd internet explorer 3.0 html//",
    "-//microsoft//dtd internet explorer 3.0 tables//",
    "-//netscape comm. corp.//dtd html//",
    "-//netscape comm. corp.//dtd strict html//",
    "-//o'reilly and associates//dtd html 2.0//",
    "-//o'reilly and associates//dtd html extended 1.0//",
    "-//o'reilly and associates//dtd html extended relaxed 1.0//",
    "-//softquad software//dtd hotmetal pro 6.0::19990601::extensions to html 4.0//",
    "-//softquad//dtd hotmetal pro 4.0::19971010::extensions to html 4.0//",
    "-//spyglass//dtd html 2.0 extended//",
    "-//sq//dtd html 2.0 hotmetal + extensions//",
    "-//sun microsystems corp.//dtd hotjava html//",
    "-//sun microsystems corp.//dtd hotjava strict html//",
    "-//w3c//dtd html 3 1995-03-24//",
    "-//w3c//dtd html 3.2 draft//",
    "-//w3c//dtd html 3.2 final//",
    "-//w3c//dtd html 3.2//",
    "-//w3c//dtd html 3.2s draft//",
    "-//w3c//dtd html 4.0 frameset//",
    "-//w3c//dtd html 4.0 transitional//",
    "-//w3c//dtd html experimental 19960712//",
    "-//w3c//dtd html experimental 970421//",
    "-//w3c//dtd html experimental 970421//",
    "-//w3c//dtd w3 html//",
    "-//w3o//dtd w3 html 3.0//",
    "-//webtechs//dtd mozilla html 2.0//",
    "-//webtechs//dtd mozilla html//",
];

/// Quirky public ID exact matches.
pub static QUIRKY_PUBLIC_MATCHES: &[&str] = &[
    "-//w3o//dtd w3 html strict 3.0//en//",
    "-/w3c/dtd html 4.0 transitional/en",
    "html",
];

/// Quirky system ID exact matches.
pub static QUIRKY_SYSTEM_MATCHES: &[&str] = &[
    "http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd",
];

/// Limited quirky public ID prefixes.
pub static LIMITED_QUIRKY_PUBLIC_PREFIXES: &[&str] = &[
    "-//w3c//dtd xhtml 1.0 frameset//",
    "-//w3c//dtd xhtml 1.0 transitional//",
];

/// HTML4 public ID prefixes.
pub static HTML4_PUBLIC_PREFIXES: &[&str] = &[
    "-//w3c//dtd html 4.01 frameset//",
    "-//w3c//dtd html 4.01 transitional//",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_void_elements_contains_br() {
        assert!(VOID_ELEMENTS.contains("br"));
        assert!(VOID_ELEMENTS.contains("img"));
        assert!(!VOID_ELEMENTS.contains("div"));
    }

    #[test]
    fn test_special_elements() {
        assert!(SPECIAL_ELEMENTS.contains("div"));
        assert!(SPECIAL_ELEMENTS.contains("table"));
        assert!(!SPECIAL_ELEMENTS.contains("span"));
    }

    #[test]
    fn test_heading_elements() {
        assert!(HEADING_ELEMENTS.contains("h1"));
        assert!(HEADING_ELEMENTS.contains("h6"));
        assert!(!HEADING_ELEMENTS.contains("h7"));
    }

    #[test]
    fn test_formatting_elements() {
        assert!(FORMATTING_ELEMENTS.contains("b"));
        assert!(FORMATTING_ELEMENTS.contains("strong"));
        assert!(!FORMATTING_ELEMENTS.contains("div"));
    }

    #[test]
    fn test_svg_adjustments() {
        assert_eq!(SVG_TAG_NAME_ADJUSTMENTS.get("foreignobject"), Some(&"foreignObject"));
        assert_eq!(SVG_ATTRIBUTE_ADJUSTMENTS.get("viewbox"), Some(&"viewBox"));
    }

    #[test]
    fn test_namespace_mapping() {
        assert_eq!(NAMESPACE_URL_TO_PREFIX.get("http://www.w3.org/1999/xhtml"), Some(&"html"));
        assert_eq!(NAMESPACE_URL_TO_PREFIX.get("http://www.w3.org/2000/svg"), Some(&"svg"));
    }

    #[test]
    fn test_html_integration_points() {
        assert!(is_html_integration_point("svg", "foreignObject"));
        assert!(is_html_integration_point("svg", "desc"));
        assert!(!is_html_integration_point("html", "div"));
    }

    #[test]
    fn test_mathml_text_integration_points() {
        assert!(is_mathml_text_integration_point("math", "mi"));
        assert!(is_mathml_text_integration_point("math", "mtext"));
        assert!(!is_mathml_text_integration_point("html", "div"));
    }

    #[test]
    fn test_scope_terminators() {
        assert!(DEFAULT_SCOPE_TERMINATORS.contains("html"));
        assert!(DEFAULT_SCOPE_TERMINATORS.contains("template"));
        assert!(BUTTON_SCOPE_TERMINATORS.contains("button"));
        assert!(!DEFAULT_SCOPE_TERMINATORS.contains("button"));
    }
}
