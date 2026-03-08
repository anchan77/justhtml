/// @file constants.cpp
/// @brief HTML5 specification constant data tables.

#include "justhtml/constants.hpp"

namespace justhtml {
namespace constants {

const std::unordered_map<std::string, ForeignAttr>& foreign_attribute_adjustments() {
    static const std::unordered_map<std::string, ForeignAttr> data = {
        {"xlink:actuate", {std::string("xlink"), "actuate", ns::XLINK}},
        {"xlink:arcrole", {std::string("xlink"), "arcrole", ns::XLINK}},
        {"xlink:href", {std::string("xlink"), "href", ns::XLINK}},
        {"xlink:role", {std::string("xlink"), "role", ns::XLINK}},
        {"xlink:show", {std::string("xlink"), "show", ns::XLINK}},
        {"xlink:title", {std::string("xlink"), "title", ns::XLINK}},
        {"xlink:type", {std::string("xlink"), "type", ns::XLINK}},
        {"xml:lang", {std::string("xml"), "lang", ns::XML}},
        {"xml:space", {std::string("xml"), "space", ns::XML}},
        {"xmlns", {std::nullopt, "xmlns", ns::XMLNS}},
        {"xmlns:xlink", {std::string("xmlns"), "xlink", ns::XMLNS}},
    };
    return data;
}

const std::unordered_map<std::string, std::string>& mathml_attribute_adjustments() {
    static const std::unordered_map<std::string, std::string> data = {
        {"definitionurl", "definitionURL"},
    };
    return data;
}

const std::unordered_map<std::string, std::string>& svg_attribute_adjustments() {
    static const std::unordered_map<std::string, std::string> data = {
        {"attributename", "attributeName"},
        {"attributetype", "attributeType"},
        {"basefrequency", "baseFrequency"},
        {"baseprofile", "baseProfile"},
        {"calcmode", "calcMode"},
        {"clippathunits", "clipPathUnits"},
        {"diffuseconstant", "diffuseConstant"},
        {"edgemode", "edgeMode"},
        {"filterunits", "filterUnits"},
        {"glyphref", "glyphRef"},
        {"gradienttransform", "gradientTransform"},
        {"gradientunits", "gradientUnits"},
        {"kernelmatrix", "kernelMatrix"},
        {"kernelunitlength", "kernelUnitLength"},
        {"keypoints", "keyPoints"},
        {"keysplines", "keySplines"},
        {"keytimes", "keyTimes"},
        {"lengthadjust", "lengthAdjust"},
        {"limitingconeangle", "limitingConeAngle"},
        {"markerheight", "markerHeight"},
        {"markerunits", "markerUnits"},
        {"markerwidth", "markerWidth"},
        {"maskcontentunits", "maskContentUnits"},
        {"maskunits", "maskUnits"},
        {"numoctaves", "numOctaves"},
        {"pathlength", "pathLength"},
        {"patterncontentunits", "patternContentUnits"},
        {"patterntransform", "patternTransform"},
        {"patternunits", "patternUnits"},
        {"pointsatx", "pointsAtX"},
        {"pointsaty", "pointsAtY"},
        {"pointsatz", "pointsAtZ"},
        {"preservealpha", "preserveAlpha"},
        {"preserveaspectratio", "preserveAspectRatio"},
        {"primitiveunits", "primitiveUnits"},
        {"refx", "refX"},
        {"refy", "refY"},
        {"repeatcount", "repeatCount"},
        {"repeatdur", "repeatDur"},
        {"requiredextensions", "requiredExtensions"},
        {"requiredfeatures", "requiredFeatures"},
        {"specularconstant", "specularConstant"},
        {"specularexponent", "specularExponent"},
        {"spreadmethod", "spreadMethod"},
        {"startoffset", "startOffset"},
        {"stddeviation", "stdDeviation"},
        {"stitchtiles", "stitchTiles"},
        {"surfacescale", "surfaceScale"},
        {"systemlanguage", "systemLanguage"},
        {"tablevalues", "tableValues"},
        {"targetx", "targetX"},
        {"targety", "targetY"},
        {"textlength", "textLength"},
        {"viewbox", "viewBox"},
        {"viewtarget", "viewTarget"},
        {"xchannelselector", "xChannelSelector"},
        {"ychannelselector", "yChannelSelector"},
        {"zoomandpan", "zoomAndPan"},
    };
    return data;
}

const std::unordered_map<std::string, std::string>& svg_tag_name_adjustments() {
    static const std::unordered_map<std::string, std::string> data = {
        {"altglyph", "altGlyph"},
        {"altglyphdef", "altGlyphDef"},
        {"altglyphitem", "altGlyphItem"},
        {"animatecolor", "animateColor"},
        {"animatemotion", "animateMotion"},
        {"animatetransform", "animateTransform"},
        {"clippath", "clipPath"},
        {"feblend", "feBlend"},
        {"fecolormatrix", "feColorMatrix"},
        {"fecomponenttransfer", "feComponentTransfer"},
        {"fecomposite", "feComposite"},
        {"feconvolvematrix", "feConvolveMatrix"},
        {"fediffuselighting", "feDiffuseLighting"},
        {"fedisplacementmap", "feDisplacementMap"},
        {"fedistantlight", "feDistantLight"},
        {"feflood", "feFlood"},
        {"fefunca", "feFuncA"},
        {"fefuncb", "feFuncB"},
        {"fefuncg", "feFuncG"},
        {"fefuncr", "feFuncR"},
        {"fegaussianblur", "feGaussianBlur"},
        {"feimage", "feImage"},
        {"femerge", "feMerge"},
        {"femergenode", "feMergeNode"},
        {"femorphology", "feMorphology"},
        {"feoffset", "feOffset"},
        {"fepointlight", "fePointLight"},
        {"fespecularlighting", "feSpecularLighting"},
        {"fespotlight", "feSpotLight"},
        {"fetile", "feTile"},
        {"feturbulence", "feTurbulence"},
        {"foreignobject", "foreignObject"},
        {"glyphref", "glyphRef"},
        {"lineargradient", "linearGradient"},
        {"radialgradient", "radialGradient"},
        {"textpath", "textPath"},
    };
    return data;
}

const std::unordered_map<std::string, std::string>& namespace_url_to_prefix() {
    static const std::unordered_map<std::string, std::string> data = {
        {ns::HTML, "html"},
        {ns::MATHML, "math"},
        {ns::SVG, "svg"},
    };
    return data;
}

const NsElementSet& html_integration_point_elements() {
    static const NsElementSet data = {
        {ns::MATHML, "annotation-xml"},
        {ns::SVG, "foreignObject"},
        {ns::SVG, "desc"},
        {ns::SVG, "title"},
    };
    return data;
}

const NsElementSet& mathml_text_integration_point_elements() {
    static const NsElementSet data = {
        {ns::MATHML, "mi"},
        {ns::MATHML, "mo"},
        {ns::MATHML, "mn"},
        {ns::MATHML, "ms"},
        {ns::MATHML, "mtext"},
    };
    return data;
}

const NsElementSet& html_integration_point_set() {
    static const NsElementSet data = {
        {"math", "annotation-xml"},
        {"svg", "foreignObject"},
        {"svg", "desc"},
        {"svg", "title"},
    };
    return data;
}

const NsElementSet& mathml_text_integration_point_set() {
    static const NsElementSet data = {
        {"math", "mi"},
        {"math", "mo"},
        {"math", "mn"},
        {"math", "ms"},
        {"math", "mtext"},
    };
    return data;
}

const std::vector<std::string>& quirky_public_prefixes() {
    static const std::vector<std::string> data = {
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
    };
    return data;
}

const std::vector<std::string>& quirky_public_matches() {
    static const std::vector<std::string> data = {
        "-//w3o//dtd w3 html strict 3.0//en//",
        "-/w3c/dtd html 4.0 transitional/en",
        "html",
    };
    return data;
}

const std::vector<std::string>& quirky_system_matches() {
    static const std::vector<std::string> data = {
        "http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd",
    };
    return data;
}

const std::vector<std::string>& limited_quirky_public_prefixes() {
    static const std::vector<std::string> data = {
        "-//w3c//dtd xhtml 1.0 frameset//",
        "-//w3c//dtd xhtml 1.0 transitional//",
    };
    return data;
}

const std::vector<std::string>& html4_public_prefixes() {
    static const std::vector<std::string> data = {
        "-//w3c//dtd html 4.01 frameset//",
        "-//w3c//dtd html 4.01 transitional//",
    };
    return data;
}

const std::unordered_set<std::string>& heading_elements() {
    static const std::unordered_set<std::string> data = {"h1", "h2", "h3", "h4", "h5", "h6"};
    return data;
}

const std::unordered_set<std::string>& formatting_elements() {
    static const std::unordered_set<std::string> data = {
        "a", "b", "big", "code", "em", "font", "i", "nobr",
        "s", "small", "strike", "strong", "tt", "u",
    };
    return data;
}

const std::unordered_set<std::string>& special_elements() {
    static const std::unordered_set<std::string> data = {
        "address",  "applet",    "area",     "article",  "aside",     "base",
        "basefont", "bgsound",   "blockquote", "body",   "br",        "button",
        "caption",  "center",    "col",      "colgroup", "dd",        "details",
        "dialog",   "dir",       "div",      "dl",       "dt",        "embed",
        "fieldset", "figcaption", "figure",  "footer",   "form",      "frame",
        "frameset", "h1",        "h2",       "h3",       "h4",        "h5",
        "h6",       "head",      "header",   "hgroup",   "hr",        "html",
        "iframe",   "img",       "input",    "keygen",   "li",        "link",
        "listing",  "main",      "marquee",  "menu",     "menuitem",  "meta",
        "nav",      "noembed",   "noframes", "noscript", "object",    "ol",
        "p",        "param",     "plaintext", "pre",     "script",    "search",
        "section",  "select",    "source",   "style",    "summary",   "table",
        "tbody",    "td",        "template", "textarea", "tfoot",     "th",
        "thead",    "title",     "tr",       "track",    "ul",        "wbr",
    };
    return data;
}

const std::unordered_set<std::string>& default_scope_terminators() {
    static const std::unordered_set<std::string> data = {
        "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
    };
    return data;
}

const std::unordered_set<std::string>& button_scope_terminators() {
    static const std::unordered_set<std::string> data = {
        "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
        "button",
    };
    return data;
}

const std::unordered_set<std::string>& list_item_scope_terminators() {
    static const std::unordered_set<std::string> data = {
        "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
        "ol", "ul",
    };
    return data;
}

const std::unordered_set<std::string>& definition_scope_terminators() {
    static const std::unordered_set<std::string> data = {
        "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
        "dl",
    };
    return data;
}

const std::unordered_set<std::string>& table_scope_terminators() {
    static const std::unordered_set<std::string> data = {"html", "table", "template"};
    return data;
}

const std::unordered_set<std::string>& table_foster_targets() {
    static const std::unordered_set<std::string> data = {
        "table", "tbody", "tfoot", "thead", "tr",
    };
    return data;
}

const std::unordered_set<std::string>& foreign_breakout_elements() {
    static const std::unordered_set<std::string> data = {
        "b",    "big",  "blockquote", "body", "br",      "center", "code",   "dd",
        "div",  "dl",   "dt",         "em",   "embed",   "h1",     "h2",     "h3",
        "h4",   "h5",   "h6",         "head", "hr",      "i",      "img",    "li",
        "listing", "menu", "meta",    "nobr", "ol",      "p",      "pre",    "ruby",
        "s",    "small", "span",      "strong", "strike", "sub",    "sup",    "table",
        "tt",   "u",    "ul",         "var",
    };
    return data;
}

const std::unordered_set<std::string>& table_allowed_children() {
    static const std::unordered_set<std::string> data = {
        "caption", "colgroup", "tbody", "tfoot", "thead", "tr",
        "td",      "th",       "script", "template", "style",
    };
    return data;
}

const std::unordered_set<std::string>& implied_end_tags() {
    static const std::unordered_set<std::string> data = {
        "dd", "dt", "li", "option", "optgroup", "p", "rb", "rp", "rt", "rtc",
    };
    return data;
}

const std::unordered_set<std::string>& void_elements() {
    static const std::unordered_set<std::string> data = {
        "area", "base", "br",    "col",   "embed", "hr",   "img",
        "input", "link", "meta", "param", "source", "track", "wbr",
    };
    return data;
}

}  // namespace constants
}  // namespace justhtml
