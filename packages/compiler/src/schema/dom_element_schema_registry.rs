//! DOM Element Schema Registry
//!
//! Corresponds to packages/compiler/src/schema/dom_element_schema_registry.ts (557 lines)
//!
//! # Security Warning
//!
//! ```text
//! =================================================================================================
//! =========== S T O P   -  S T O P   -  S T O P   -  S T O P   -  S T O P   -  S T O P  ===========
//! =================================================================================================
//!
//!                       DO NOT EDIT THIS DOM SCHEMA WITHOUT A SECURITY REVIEW!
//!
//! Newly added properties must be security reviewed and assigned an appropriate SecurityContext in
//! dom_security_schema.rs. Reach out to mprobst & rjamet for details.
//!
//! =================================================================================================
//! ```

use super::dom_security_schema::security_schema;
use super::element_schema_registry::{
    ElementSchemaRegistry, NormalizationResult, ValidationResult,
};
use crate::core::{SchemaMetadata, SecurityContext};
use crate::ml_parser::tags::{is_ng_container, is_ng_content};
use crate::util::dash_case_to_camel_case;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

// Property type constants
const BOOLEAN: &str = "boolean";
const NUMBER: &str = "number";
const STRING: &str = "string";
const OBJECT: &str = "object";

/// DOM schema encoding inheritance, properties, and events
///
/// ## Format:
/// Each line: `element_inheritance|properties`
///
/// - Elements separated by `,` have identical properties
/// - `^parentElement` indicates inheritance
/// - Property prefixes:
///   - (no prefix): string property
///   - `*`: event
///   - `!`: boolean
///   - `#`: number
///   - `%`: object
///
/// Full DOM schema from Angular (188 entries)
pub static SCHEMA: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "[Element]|textContent,%ariaActiveDescendantElement,%ariaAtomic,%ariaAutoComplete,%ariaBusy,%ariaChecked,%ariaColCount,%ariaColIndex,%ariaColIndexText,%ariaColSpan,%ariaControlsElements,%ariaCurrent,%ariaDescribedByElements,%ariaDescription,%ariaDetailsElements,%ariaDisabled,%ariaErrorMessageElements,%ariaExpanded,%ariaFlowToElements,%ariaHasPopup,%ariaHidden,%ariaInvalid,%ariaKeyShortcuts,%ariaLabel,%ariaLabelledByElements,%ariaLevel,%ariaLive,%ariaModal,%ariaMultiLine,%ariaMultiSelectable,%ariaOrientation,%ariaOwnsElements,%ariaPlaceholder,%ariaPosInSet,%ariaPressed,%ariaReadOnly,%ariaRelevant,%ariaRequired,%ariaRoleDescription,%ariaRowCount,%ariaRowIndex,%ariaRowIndexText,%ariaRowSpan,%ariaSelected,%ariaSetSize,%ariaSort,%ariaValueMax,%ariaValueMin,%ariaValueNow,%ariaValueText,%classList,className,elementTiming,id,innerHTML,*beforecopy,*beforecut,*beforepaste,*fullscreenchange,*fullscreenerror,*search,*webkitfullscreenchange,*webkitfullscreenerror,outerHTML,%part,#scrollLeft,#scrollTop,slot,*message,*mozfullscreenchange,*mozfullscreenerror,*mozpointerlockchange,*mozpointerlockerror,*webglcontextcreationerror,*webglcontextlost,*webglcontextrestored",
        "[HTMLElement]^[Element]|accessKey,autocapitalize,!autofocus,contentEditable,dir,!draggable,enterKeyHint,!hidden,!inert,innerText,inputMode,lang,nonce,*abort,*animationend,*animationiteration,*animationstart,*auxclick,*beforexrselect,*blur,*cancel,*canplay,*canplaythrough,*change,*click,*close,*contextmenu,*copy,*cuechange,*cut,*dblclick,*drag,*dragend,*dragenter,*dragleave,*dragover,*dragstart,*drop,*durationchange,*emptied,*ended,*error,*focus,*formdata,*gotpointercapture,*input,*invalid,*keydown,*keypress,*keyup,*load,*loadeddata,*loadedmetadata,*loadstart,*lostpointercapture,*mousedown,*mouseenter,*mouseleave,*mousemove,*mouseout,*mouseover,*mouseup,*mousewheel,*paste,*pause,*play,*playing,*pointercancel,*pointerdown,*pointerenter,*pointerleave,*pointermove,*pointerout,*pointerover,*pointerrawupdate,*pointerup,*progress,*ratechange,*reset,*resize,*scroll,*securitypolicyviolation,*seeked,*seeking,*select,*selectionchange,*selectstart,*slotchange,*stalled,*submit,*suspend,*timeupdate,*toggle,*transitioncancel,*transitionend,*transitionrun,*transitionstart,*volumechange,*waiting,*webkitanimationend,*webkitanimationiteration,*webkitanimationstart,*webkittransitionend,*wheel,outerText,!spellcheck,%style,#tabIndex,title,!translate,virtualKeyboardPolicy",
        "abbr,address,article,aside,b,bdi,bdo,cite,content,code,dd,dfn,dt,em,figcaption,figure,footer,header,hgroup,i,kbd,main,mark,nav,noscript,rb,rp,rt,rtc,ruby,s,samp,search,section,small,strong,sub,sup,u,var,wbr^[HTMLElement]|accessKey,autocapitalize,!autofocus,contentEditable,dir,!draggable,enterKeyHint,!hidden,innerText,inputMode,lang,nonce,*abort,*animationend,*animationiteration,*animationstart,*auxclick,*beforexrselect,*blur,*cancel,*canplay,*canplaythrough,*change,*click,*close,*contextmenu,*copy,*cuechange,*cut,*dblclick,*drag,*dragend,*dragenter,*dragleave,*dragover,*dragstart,*drop,*durationchange,*emptied,*ended,*error,*focus,*formdata,*gotpointercapture,*input,*invalid,*keydown,*keypress,*keyup,*load,*loadeddata,*loadedmetadata,*loadstart,*lostpointercapture,*mousedown,*mouseenter,*mouseleave,*mousemove,*mouseout,*mouseover,*mouseup,*mousewheel,*paste,*pause,*play,*playing,*pointercancel,*pointerdown,*pointerenter,*pointerleave,*pointermove,*pointerout,*pointerover,*pointerrawupdate,*pointerup,*progress,*ratechange,*reset,*resize,*scroll,*securitypolicyviolation,*seeked,*seeking,*select,*selectionchange,*selectstart,*slotchange,*stalled,*submit,*suspend,*timeupdate,*toggle,*transitioncancel,*transitionend,*transitionrun,*transitionstart,*volumechange,*waiting,*webkitanimationend,*webkitanimationiteration,*webkitanimationstart,*webkittransitionend,*wheel,outerText,!spellcheck,%style,#tabIndex,title,!translate,virtualKeyboardPolicy",
        "media^[HTMLElement]|!autoplay,!controls,%controlsList,%crossOrigin,#currentTime,!defaultMuted,#defaultPlaybackRate,!disableRemotePlayback,!loop,!muted,*encrypted,*waitingforkey,#playbackRate,preload,!preservesPitch,src,%srcObject,#volume",
        ":svg:^[HTMLElement]|!autofocus,nonce,*abort,*animationend,*animationiteration,*animationstart,*auxclick,*beforexrselect,*blur,*cancel,*canplay,*canplaythrough,*change,*click,*close,*contextmenu,*copy,*cuechange,*cut,*dblclick,*drag,*dragend,*dragenter,*dragleave,*dragover,*dragstart,*drop,*durationchange,*emptied,*ended,*error,*focus,*formdata,*gotpointercapture,*input,*invalid,*keydown,*keypress,*keyup,*load,*loadeddata,*loadedmetadata,*loadstart,*lostpointercapture,*mousedown,*mouseenter,*mouseleave,*mousemove,*mouseout,*mouseover,*mouseup,*mousewheel,*paste,*pause,*play,*playing,*pointercancel,*pointerdown,*pointerenter,*pointerleave,*pointermove,*pointerout,*pointerover,*pointerrawupdate,*pointerup,*progress,*ratechange,*reset,*resize,*scroll,*securitypolicyviolation,*seeked,*seeking,*select,*selectionchange,*selectstart,*slotchange,*stalled,*submit,*suspend,*timeupdate,*toggle,*transitioncancel,*transitionend,*transitionrun,*transitionstart,*volumechange,*waiting,*webkitanimationend,*webkitanimationiteration,*webkitanimationstart,*webkittransitionend,*wheel,%style,#tabIndex",
        ":svg:graphics^:svg:|",
        ":svg:animation^:svg:|*begin,*end,*repeat",
        ":svg:geometry^:svg:|",
        ":svg:componentTransferFunction^:svg:|",
        ":svg:gradient^:svg:|",
        ":svg:textContent^:svg:graphics|",
        ":svg:textPositioning^:svg:textContent|",
        "a^[HTMLElement]|charset,coords,download,hash,host,hostname,href,hreflang,name,password,pathname,ping,port,protocol,referrerPolicy,rel,%relList,rev,search,shape,target,text,type,username",
        "area^[HTMLElement]|alt,coords,download,hash,host,hostname,href,!noHref,password,pathname,ping,port,protocol,referrerPolicy,rel,%relList,search,shape,target,username",
        "audio^media|",
        "br^[HTMLElement]|clear",
        "base^[HTMLElement]|href,target",
        "body^[HTMLElement]|aLink,background,bgColor,link,*afterprint,*beforeprint,*beforeunload,*blur,*error,*focus,*hashchange,*languagechange,*load,*message,*messageerror,*offline,*online,*pagehide,*pageshow,*popstate,*rejectionhandled,*resize,*scroll,*storage,*unhandledrejection,*unload,text,vLink",
        "button^[HTMLElement]|!disabled,formAction,formEnctype,formMethod,!formNoValidate,formTarget,name,type,value",
        "canvas^[HTMLElement]|#height,#width",
        "content^[HTMLElement]|select",
        "dl^[HTMLElement]|!compact",
        "data^[HTMLElement]|value",
        "datalist^[HTMLElement]|",
        "details^[HTMLElement]|!open",
        "dialog^[HTMLElement]|!open,returnValue",
        "dir^[HTMLElement]|!compact",
        "div^[HTMLElement]|align",
        "embed^[HTMLElement]|align,height,name,src,type,width",
        "fieldset^[HTMLElement]|!disabled,name",
        "font^[HTMLElement]|color,face,size",
        "form^[HTMLElement]|acceptCharset,action,autocomplete,encoding,enctype,method,name,!noValidate,target",
        "frame^[HTMLElement]|frameBorder,longDesc,marginHeight,marginWidth,name,!noResize,scrolling,src",
        "frameset^[HTMLElement]|cols,*afterprint,*beforeprint,*beforeunload,*blur,*error,*focus,*hashchange,*languagechange,*load,*message,*messageerror,*offline,*online,*pagehide,*pageshow,*popstate,*rejectionhandled,*resize,*scroll,*storage,*unhandledrejection,*unload,rows",
        "hr^[HTMLElement]|align,color,!noShade,size,width",
        "head^[HTMLElement]|",
        "h1,h2,h3,h4,h5,h6^[HTMLElement]|align",
        "html^[HTMLElement]|version",
        "iframe^[HTMLElement]|align,allow,!allowFullscreen,!allowPaymentRequest,csp,frameBorder,height,loading,longDesc,marginHeight,marginWidth,name,referrerPolicy,%sandbox,scrolling,src,srcdoc,width",
        "img^[HTMLElement]|align,alt,border,%crossOrigin,decoding,#height,#hspace,!isMap,loading,longDesc,lowsrc,name,referrerPolicy,sizes,src,srcset,useMap,#vspace,#width",
        "input^[HTMLElement]|accept,align,alt,autocomplete,!checked,!defaultChecked,defaultValue,dirName,!disabled,%files,formAction,formEnctype,formMethod,!formNoValidate,formTarget,#height,!incremental,!indeterminate,max,#maxLength,min,#minLength,!multiple,name,pattern,placeholder,!readOnly,!required,selectionDirection,#selectionEnd,#selectionStart,#size,src,step,type,useMap,value,%valueAsDate,#valueAsNumber,#width",
        "li^[HTMLElement]|type,#value",
        "label^[HTMLElement]|htmlFor",
        "legend^[HTMLElement]|align",
        "link^[HTMLElement]|as,charset,%crossOrigin,!disabled,href,hreflang,imageSizes,imageSrcset,integrity,media,referrerPolicy,rel,%relList,rev,%sizes,target,type",
        "map^[HTMLElement]|name",
        "marquee^[HTMLElement]|behavior,bgColor,direction,height,#hspace,#loop,#scrollAmount,#scrollDelay,!trueSpeed,#vspace,width",
        "menu^[HTMLElement]|!compact",
        "meta^[HTMLElement]|content,httpEquiv,media,name,scheme",
        "meter^[HTMLElement]|#high,#low,#max,#min,#optimum,#value",
        "ins,del^[HTMLElement]|cite,dateTime",
        "ol^[HTMLElement]|!compact,!reversed,#start,type",
        "object^[HTMLElement]|align,archive,border,code,codeBase,codeType,data,!declare,height,#hspace,name,standby,type,useMap,#vspace,width",
        "optgroup^[HTMLElement]|!disabled,label",
        "option^[HTMLElement]|!defaultSelected,!disabled,label,!selected,text,value",
        "output^[HTMLElement]|defaultValue,%htmlFor,name,value",
        "p^[HTMLElement]|align",
        "param^[HTMLElement]|name,type,value,valueType",
        "picture^[HTMLElement]|",
        "pre^[HTMLElement]|#width",
        "progress^[HTMLElement]|#max,#value",
        "q,blockquote,cite^[HTMLElement]|",
        "script^[HTMLElement]|!async,charset,%crossOrigin,!defer,event,htmlFor,integrity,!noModule,%referrerPolicy,src,text,type",
        "select^[HTMLElement]|autocomplete,!disabled,#length,!multiple,name,!required,#selectedIndex,#size,value",
        "selectedcontent^[HTMLElement]|",
        "slot^[HTMLElement]|name",
        "source^[HTMLElement]|#height,media,sizes,src,srcset,type,#width",
        "span^[HTMLElement]|",
        "style^[HTMLElement]|!disabled,media,type",
        "search^[HTMLELement]|",
        "caption^[HTMLElement]|align",
        "th,td^[HTMLElement]|abbr,align,axis,bgColor,ch,chOff,#colSpan,headers,height,!noWrap,#rowSpan,scope,vAlign,width",
        "col,colgroup^[HTMLElement]|align,ch,chOff,#span,vAlign,width",
        "table^[HTMLElement]|align,bgColor,border,%caption,cellPadding,cellSpacing,frame,rules,summary,%tFoot,%tHead,width",
        "tr^[HTMLElement]|align,bgColor,ch,chOff,vAlign",
        "tfoot,thead,tbody^[HTMLElement]|align,ch,chOff,vAlign",
        "template^[HTMLElement]|",
        "textarea^[HTMLElement]|autocomplete,#cols,defaultValue,dirName,!disabled,#maxLength,#minLength,name,placeholder,!readOnly,!required,#rows,selectionDirection,#selectionEnd,#selectionStart,value,wrap",
        "time^[HTMLElement]|dateTime",
        "title^[HTMLElement]|text",
        "track^[HTMLElement]|!default,kind,label,src,srclang",
        "ul^[HTMLElement]|!compact,type",
        "unknown^[HTMLElement]|",
        "video^media|!disablePictureInPicture,#height,*enterpictureinpicture,*leavepictureinpicture,!playsInline,poster,#width",
        ":svg:a^:svg:graphics|",
        ":svg:animate^:svg:animation|",
        ":svg:animateMotion^:svg:animation|",
        ":svg:animateTransform^:svg:animation|",
        ":svg:circle^:svg:geometry|",
        ":svg:clipPath^:svg:graphics|",
        ":svg:defs^:svg:graphics|",
        ":svg:desc^:svg:|",
        ":svg:discard^:svg:|",
        ":svg:ellipse^:svg:geometry|",
        ":svg:feBlend^:svg:|",
        ":svg:feColorMatrix^:svg:|",
        ":svg:feComponentTransfer^:svg:|",
        ":svg:feComposite^:svg:|",
        ":svg:feConvolveMatrix^:svg:|",
        ":svg:feDiffuseLighting^:svg:|",
        ":svg:feDisplacementMap^:svg:|",
        ":svg:feDistantLight^:svg:|",
        ":svg:feDropShadow^:svg:|",
        ":svg:feFlood^:svg:|",
        ":svg:feFuncA^:svg:componentTransferFunction|",
        ":svg:feFuncB^:svg:componentTransferFunction|",
        ":svg:feFuncG^:svg:componentTransferFunction|",
        ":svg:feFuncR^:svg:componentTransferFunction|",
        ":svg:feGaussianBlur^:svg:|",
        ":svg:feImage^:svg:|",
        ":svg:feMerge^:svg:|",
        ":svg:feMergeNode^:svg:|",
        ":svg:feMorphology^:svg:|",
        ":svg:feOffset^:svg:|",
        ":svg:fePointLight^:svg:|",
        ":svg:feSpecularLighting^:svg:|",
        ":svg:feSpotLight^:svg:|",
        ":svg:feTile^:svg:|",
        ":svg:feTurbulence^:svg:|",
        ":svg:filter^:svg:|",
        ":svg:foreignObject^:svg:graphics|",
        ":svg:g^:svg:graphics|",
        ":svg:image^:svg:graphics|decoding",
        ":svg:line^:svg:geometry|",
        ":svg:linearGradient^:svg:gradient|",
        ":svg:mpath^:svg:|",
        ":svg:marker^:svg:|",
        ":svg:mask^:svg:|",
        ":svg:metadata^:svg:|",
        ":svg:path^:svg:geometry|",
        ":svg:pattern^:svg:|",
        ":svg:polygon^:svg:geometry|",
        ":svg:polyline^:svg:geometry|",
        ":svg:radialGradient^:svg:gradient|",
        ":svg:rect^:svg:geometry|",
        ":svg:svg^:svg:graphics|#currentScale,#zoomAndPan",
        ":svg:script^:svg:|type",
        ":svg:set^:svg:animation|",
        ":svg:stop^:svg:|",
        ":svg:style^:svg:|!disabled,media,title,type",
        ":svg:switch^:svg:graphics|",
        ":svg:symbol^:svg:|",
        ":svg:tspan^:svg:textPositioning|",
        ":svg:text^:svg:textPositioning|",
        ":svg:textPath^:svg:textContent|",
        ":svg:title^:svg:|",
        ":svg:use^:svg:graphics|",
        ":svg:view^:svg:|#zoomAndPan",
        "data^[HTMLElement]|value",
        "keygen^[HTMLElement]|!autofocus,challenge,!disabled,form,keytype,name",
        "menuitem^[HTMLElement]|type,label,icon,!disabled,!checked,radiogroup,!default",
        "summary^[HTMLElement]|",
        "time^[HTMLElement]|dateTime",
        ":svg:cursor^:svg:|",
        ":math:^[HTMLElement]|!autofocus,nonce,*abort,*animationend,*animationiteration,*animationstart,*auxclick,*beforeinput,*beforematch,*beforetoggle,*beforexrselect,*blur,*cancel,*canplay,*canplaythrough,*change,*click,*close,*contentvisibilityautostatechange,*contextlost,*contextmenu,*contextrestored,*copy,*cuechange,*cut,*dblclick,*drag,*dragend,*dragenter,*dragleave,*dragover,*dragstart,*drop,*durationchange,*emptied,*ended,*error,*focus,*formdata,*gotpointercapture,*input,*invalid,*keydown,*keypress,*keyup,*load,*loadeddata,*loadedmetadata,*loadstart,*lostpointercapture,*mousedown,*mouseenter,*mouseleave,*mousemove,*mouseout,*mouseover,*mouseup,*mousewheel,*paste,*pause,*play,*playing,*pointercancel,*pointerdown,*pointerenter,*pointerleave,*pointermove,*pointerout,*pointerover,*pointerrawupdate,*pointerup,*progress,*ratechange,*reset,*resize,*scroll,*scrollend,*securitypolicyviolation,*seeked,*seeking,*select,*selectionchange,*selectstart,*slotchange,*stalled,*submit,*suspend,*timeupdate,*toggle,*transitioncancel,*transitionend,*transitionrun,*transitionstart,*volumechange,*waiting,*webkitanimationend,*webkitanimationiteration,*webkitanimationstart,*webkittransitionend,*wheel,%style,#tabIndex",
        ":math:math^:math:|",
        ":math:maction^:math:|",
        ":math:menclose^:math:|",
        ":math:merror^:math:|",
        ":math:mfenced^:math:|",
        ":math:mfrac^:math:|",
        ":math:mi^:math:|",
        ":math:mmultiscripts^:math:|",
        ":math:mn^:math:|",
        ":math:mo^:math:|",
        ":math:mover^:math:|",
        ":math:mpadded^:math:|",
        ":math:mphantom^:math:|",
        ":math:mroot^:math:|",
        ":math:mrow^:math:|",
        ":math:ms^:math:|",
        ":math:mspace^:math:|",
        ":math:msqrt^:math:|",
        ":math:mstyle^:math:|",
        ":math:msub^:math:|",
        ":math:msubsup^:math:|",
        ":math:msup^:math:|",
        ":math:mtable^:math:|",
        ":math:mtd^:math:|",
        ":math:mtext^:math:|",
        ":math:mtr^:math:|",
        ":math:munder^:math:|",
        ":math:munderover^:math:|",
        ":math:semantics^:math:|",
    ]
});

/// Map from attribute names to property names
/// Full mapping from Angular (52 entries)
pub static ATTR_TO_PROP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Basic HTML attributes
    map.insert("class", "className");
    map.insert("for", "htmlFor");
    map.insert("formaction", "formAction");
    map.insert("innerHtml", "innerHTML");
    map.insert("readonly", "readOnly");
    map.insert("tabindex", "tabIndex");

    // ARIA attributes (https://www.w3.org/TR/wai-aria-1.3/#accessibilityroleandproperties-correspondence)
    map.insert("aria-activedescendant", "ariaActiveDescendantElement");
    map.insert("aria-atomic", "ariaAtomic");
    map.insert("aria-autocomplete", "ariaAutoComplete");
    map.insert("aria-busy", "ariaBusy");
    map.insert("aria-checked", "ariaChecked");
    map.insert("aria-colcount", "ariaColCount");
    map.insert("aria-colindex", "ariaColIndex");
    map.insert("aria-colindextext", "ariaColIndexText");
    map.insert("aria-colspan", "ariaColSpan");
    map.insert("aria-controls", "ariaControlsElements");
    map.insert("aria-current", "ariaCurrent");
    map.insert("aria-describedby", "ariaDescribedByElements");
    map.insert("aria-description", "ariaDescription");
    map.insert("aria-details", "ariaDetailsElements");
    map.insert("aria-disabled", "ariaDisabled");
    map.insert("aria-errormessage", "ariaErrorMessageElements");
    map.insert("aria-expanded", "ariaExpanded");
    map.insert("aria-flowto", "ariaFlowToElements");
    map.insert("aria-haspopup", "ariaHasPopup");
    map.insert("aria-hidden", "ariaHidden");
    map.insert("aria-invalid", "ariaInvalid");
    map.insert("aria-keyshortcuts", "ariaKeyShortcuts");
    map.insert("aria-label", "ariaLabel");
    map.insert("aria-labelledby", "ariaLabelledByElements");
    map.insert("aria-level", "ariaLevel");
    map.insert("aria-live", "ariaLive");
    map.insert("aria-modal", "ariaModal");
    map.insert("aria-multiline", "ariaMultiLine");
    map.insert("aria-multiselectable", "ariaMultiSelectable");
    map.insert("aria-orientation", "ariaOrientation");
    map.insert("aria-owns", "ariaOwnsElements");
    map.insert("aria-placeholder", "ariaPlaceholder");
    map.insert("aria-posinset", "ariaPosInSet");
    map.insert("aria-pressed", "ariaPressed");
    map.insert("aria-readonly", "ariaReadOnly");
    map.insert("aria-required", "ariaRequired");
    map.insert("aria-roledescription", "ariaRoleDescription");
    map.insert("aria-rowcount", "ariaRowCount");
    map.insert("aria-rowindex", "ariaRowIndex");
    map.insert("aria-rowindextext", "ariaRowIndexText");
    map.insert("aria-rowspan", "ariaRowSpan");
    map.insert("aria-selected", "ariaSelected");
    map.insert("aria-setsize", "ariaSetSize");
    map.insert("aria-sort", "ariaSort");
    map.insert("aria-valuemax", "ariaValueMax");
    map.insert("aria-valuemin", "ariaValueMin");
    map.insert("aria-valuenow", "ariaValueNow");
    map.insert("aria-valuetext", "ariaValueText");

    map
});

/// Inverted map from property names to attribute names
static PROP_TO_ATTR: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| ATTR_TO_PROP.iter().map(|(k, v)| (*v, *k)).collect());

/// NO_ERRORS_SCHEMA constant
fn no_errors_schema_name() -> &'static str {
    "no-errors-schema"
}

/// CUSTOM_ELEMENTS_SCHEMA constant
fn custom_elements_schema_name() -> &'static str {
    "custom-elements"
}

/// DOM Element Schema Registry
pub struct DomElementSchemaRegistry {
    schema: std::sync::Arc<HashMap<String, HashMap<String, String>>>,
    // We don't allow binding to events for security reasons. Allowing event bindings would almost
    // certainly introduce bad XSS vulnerabilities. Instead, we store events in a separate schema.
    event_schema: std::sync::Arc<HashMap<String, HashSet<String>>>,
}

type ParsedSchema = (
    HashMap<String, HashMap<String, String>>,
    HashMap<String, HashSet<String>>,
);

static PARSED_DOM_SCHEMA: Lazy<ParsedSchema> = Lazy::new(|| {
    let mut schema = HashMap::new();
    let mut event_schema = HashMap::new();

    // Parse SCHEMA array
    for encoded_type in SCHEMA.iter() {
        let mut properties_map: HashMap<String, String> = HashMap::new();
        let mut events_set: HashSet<String> = HashSet::new();

        // Split by '|' to get type and properties
        let parts: Vec<&str> = encoded_type.split('|').collect();
        if parts.len() != 2 {
            continue;
        }

        let str_type = parts[0];
        let str_properties = parts[1];

        // Split type to get element names and parent
        let type_parts: Vec<&str> = str_type.split('^').collect();
        let type_names = type_parts[0];
        let super_name = type_parts.get(1).copied();

        // Register all element names with same schema
        for tag in type_names.split(',') {
            let tag_lower = tag.to_lowercase();
            schema.insert(tag_lower.clone(), properties_map.clone());
            event_schema.insert(tag_lower, events_set.clone());
        }

        // Inherit from parent if specified
        if let Some(super_name) = super_name {
            let super_lower = super_name.to_lowercase();
            if let Some(super_type) = schema.get(&super_lower) {
                for (prop, value) in super_type {
                    properties_map.insert(prop.clone(), value.clone());
                }
            }
            if let Some(super_events) = event_schema.get(&super_lower) {
                for event in super_events {
                    events_set.insert(event.clone());
                }
            }
        }

        // Parse properties
        for property in str_properties.split(',') {
            if property.is_empty() {
                continue;
            }

            match property.chars().next() {
                Some('*') => {
                    // Event
                    events_set.insert(property[1..].to_string());
                }
                Some('!') => {
                    // Boolean
                    properties_map.insert(property[1..].to_string(), BOOLEAN.to_string());
                }
                Some('#') => {
                    // Number
                    properties_map.insert(property[1..].to_string(), NUMBER.to_string());
                }
                Some('%') => {
                    // Object
                    properties_map.insert(property[1..].to_string(), OBJECT.to_string());
                }
                _ => {
                    // String (default)
                    properties_map.insert(property.to_string(), STRING.to_string());
                }
            }
        }

        // Update the schema with inherited properties
        for tag in type_names.split(',') {
            let tag_lower = tag.to_lowercase();
            schema.insert(tag_lower.clone(), properties_map.clone());
            event_schema.insert(tag_lower, events_set.clone());
        }
    }

    (schema, event_schema)
});

impl DomElementSchemaRegistry {
    pub fn new() -> Self {
        let (schema, event_schema) = &*PARSED_DOM_SCHEMA;
        DomElementSchemaRegistry {
            schema: std::sync::Arc::new(schema.clone()),
            event_schema: std::sync::Arc::new(event_schema.clone()),
        }
    }

    /// Get all known attributes of an element
    pub fn all_known_attributes_of_element(&self, tag_name: &str) -> Vec<String> {
        let element_properties = self
            .schema
            .get(&tag_name.to_lowercase())
            .or_else(|| self.schema.get("unknown"));

        match element_properties {
            Some(props) => props
                .keys()
                .map(|prop| {
                    PROP_TO_ATTR
                        .get(prop.as_str())
                        .copied()
                        .unwrap_or(prop.as_str())
                        .to_string()
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get all known events of an element
    pub fn all_known_events_of_element(&self, tag_name: &str) -> Vec<String> {
        self.event_schema
            .get(&tag_name.to_lowercase())
            .map(|events| events.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for DomElementSchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementSchemaRegistry for DomElementSchemaRegistry {
    fn has_property(
        &self,
        tag_name: &str,
        prop_name: &str,
        schema_metas: &[SchemaMetadata],
    ) -> bool {
        // NO_ERRORS_SCHEMA allows all properties
        if schema_metas
            .iter()
            .any(|s| s.name == no_errors_schema_name())
        {
            return true;
        }

        // Handle custom elements (with hyphen in name)
        if tag_name.contains('-') {
            if is_ng_container(tag_name) || is_ng_content(tag_name) {
                return false;
            }

            if schema_metas
                .iter()
                .any(|s| s.name == custom_elements_schema_name())
            {
                // Can't tell now as we don't know which properties a custom element will get
                // once it is instantiated
                return true;
            }
        }

        let element_properties = self
            .schema
            .get(&tag_name.to_lowercase())
            .or_else(|| self.schema.get("unknown"));

        element_properties
            .map(|props| props.contains_key(prop_name))
            .unwrap_or(false)
    }

    fn has_element(&self, tag_name: &str, schema_metas: &[SchemaMetadata]) -> bool {
        // NO_ERRORS_SCHEMA allows all elements
        if schema_metas
            .iter()
            .any(|s| s.name == no_errors_schema_name())
        {
            return true;
        }

        // Handle custom elements
        if tag_name.contains('-') {
            if is_ng_container(tag_name) || is_ng_content(tag_name) {
                return true;
            }

            if schema_metas
                .iter()
                .any(|s| s.name == custom_elements_schema_name())
            {
                // Allow any custom elements
                return true;
            }
        }

        self.schema.contains_key(&tag_name.to_lowercase())
    }

    fn security_context(
        &self,
        element_name: &str,
        prop_name: &str,
        is_attribute: bool,
    ) -> SecurityContext {
        let prop_name = if is_attribute {
            // NB: For security purposes, use the mapped property name, not the attribute name.
            self.get_mapped_prop_name(prop_name)
        } else {
            prop_name.to_string()
        };

        // Make sure comparisons are case insensitive, so that case differences between attribute and
        // property names do not have a security impact.
        let tag_lower = element_name.to_lowercase();
        let prop_lower = prop_name.to_lowercase();

        let schema = security_schema();

        // Check tag-specific context
        let key = format!("{}|{}", tag_lower, prop_lower);
        if let Some(ctx) = schema.get(&key) {
            return *ctx;
        }

        // Check wildcard context
        let wildcard_key = format!("*|{}", prop_lower);
        schema
            .get(&wildcard_key)
            .copied()
            .unwrap_or(SecurityContext::NONE)
    }

    fn all_known_element_names(&self) -> Vec<String> {
        self.schema.keys().cloned().collect()
    }

    fn get_mapped_prop_name(&self, prop_name: &str) -> String {
        ATTR_TO_PROP
            .get(prop_name)
            .copied()
            .unwrap_or(prop_name)
            .to_string()
    }

    fn get_default_component_element_name(&self) -> String {
        "ng-component".to_string()
    }

    fn validate_property(&self, name: &str) -> ValidationResult {
        if name.to_lowercase().starts_with("on") {
            let msg = format!(
                "Binding to event property '{}' is disallowed for security reasons, \
                please use ({})=...\n\
                If '{}' is a directive input, make sure the directive is imported by the current module.",
                name,
                &name[2..],
                name
            );
            ValidationResult {
                error: true,
                msg: Some(msg),
            }
        } else {
            ValidationResult {
                error: false,
                msg: None,
            }
        }
    }

    fn validate_attribute(&self, name: &str) -> ValidationResult {
        if name.to_lowercase().starts_with("on") {
            let msg = format!(
                "Binding to event attribute '{}' is disallowed for security reasons, \
                please use ({})=...",
                name,
                &name[2..]
            );
            ValidationResult {
                error: true,
                msg: Some(msg),
            }
        } else {
            ValidationResult {
                error: false,
                msg: None,
            }
        }
    }

    fn normalize_animation_style_property(&self, prop_name: &str) -> String {
        dash_case_to_camel_case(prop_name)
    }

    fn normalize_animation_style_value(
        &self,
        camel_case_prop: &str,
        user_provided_prop: &str,
        val: &str,
    ) -> NormalizationResult {
        let mut unit = String::new();
        let str_val = val.trim();
        let mut error_msg = String::new();

        if is_pixel_dimension_style(camel_case_prop) && val != "0" && !val.is_empty() {
            // Check if it's a pure number
            if val.parse::<f64>().is_ok() {
                unit = "px".to_string();
            } else {
                // Check if value has no unit suffix
                let re = regex::Regex::new(r"^[+-]?[\d\.]+([a-z]*)$").unwrap();
                if let Some(caps) = re.captures(val) {
                    if caps.get(1).map(|m| m.as_str()).unwrap_or("").is_empty() {
                        error_msg = format!(
                            "Please provide a CSS unit value for {}:{}",
                            user_provided_prop, val
                        );
                    }
                }
            }
        }

        NormalizationResult {
            error: error_msg,
            value: format!("{}{}", str_val, unit),
        }
    }
}

/// Check if a CSS property should have pixel units by default
fn is_pixel_dimension_style(prop: &str) -> bool {
    matches!(
        prop,
        "width"
            | "height"
            | "minWidth"
            | "minHeight"
            | "maxWidth"
            | "maxHeight"
            | "left"
            | "top"
            | "bottom"
            | "right"
            | "fontSize"
            | "outlineWidth"
            | "outlineOffset"
            | "paddingTop"
            | "paddingLeft"
            | "paddingBottom"
            | "paddingRight"
            | "marginTop"
            | "marginLeft"
            | "marginBottom"
            | "marginRight"
            | "borderRadius"
            | "borderWidth"
            | "borderTopWidth"
            | "borderLeftWidth"
            | "borderRightWidth"
            | "borderBottomWidth"
            | "textIndent"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = DomElementSchemaRegistry::new();
        assert!(!registry.schema.is_empty());
        assert!(!registry.event_schema.is_empty());
    }

    #[test]
    fn test_has_element() {
        let registry = DomElementSchemaRegistry::new();
        assert!(registry.has_element("div", &[]));
        assert!(registry.has_element("a", &[]));
    }

    #[test]
    fn test_has_property() {
        let registry = DomElementSchemaRegistry::new();
        // Note: This test uses sample data, will work fully when user adds complete SCHEMA
        assert!(registry.has_property("div", "id", &[]));
    }

    #[test]
    fn test_get_mapped_prop_name() {
        let registry = DomElementSchemaRegistry::new();
        assert_eq!(registry.get_mapped_prop_name("class"), "className");
        assert_eq!(registry.get_mapped_prop_name("for"), "htmlFor");
        assert_eq!(registry.get_mapped_prop_name("unknown"), "unknown");
    }

    #[test]
    fn test_validate_property_with_on_prefix() {
        let registry = DomElementSchemaRegistry::new();
        let result = registry.validate_property("onclick");
        assert!(result.error);
        assert!(result.msg.is_some());
    }

    #[test]
    fn test_validate_property_without_on_prefix() {
        let registry = DomElementSchemaRegistry::new();
        let result = registry.validate_property("href");
        assert!(!result.error);
    }

    #[test]
    fn test_is_pixel_dimension_style() {
        assert!(is_pixel_dimension_style("width"));
        assert!(is_pixel_dimension_style("height"));
        assert!(is_pixel_dimension_style("marginTop"));
        assert!(!is_pixel_dimension_style("color"));
        assert!(!is_pixel_dimension_style("display"));
    }

    #[test]
    fn test_normalize_animation_style_value_with_number() {
        let registry = DomElementSchemaRegistry::new();
        let result = registry.normalize_animation_style_value("width", "width", "100");
        assert_eq!(result.value, "100px");
        assert!(result.error.is_empty());
    }

    #[test]
    fn test_normalize_animation_style_value_with_unit() {
        let registry = DomElementSchemaRegistry::new();
        let result = registry.normalize_animation_style_value("width", "width", "100em");
        assert_eq!(result.value, "100em");
        assert!(result.error.is_empty());
    }

    #[test]
    fn test_get_default_component_element_name() {
        let registry = DomElementSchemaRegistry::new();
        assert_eq!(
            registry.get_default_component_element_name(),
            "ng-component"
        );
    }
}
