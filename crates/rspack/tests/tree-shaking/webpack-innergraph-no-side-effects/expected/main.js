(self['webpackChunkwebpack'] = self['webpackChunkwebpack'] || []).push([["main"], {
"./index.js": function (__unused_webpack_module, exports, __webpack_require__) {
it("should be able to load package without side effects where modules are unused", ()=>{
    __webpack_require__(/* ./module */"./module.js");
});
},
"./module.js": function (__unused_webpack_module, __webpack_exports__, __webpack_require__) {
'use strict';
__webpack_require__.r(__webpack_exports__);
__webpack_require__.d(__webpack_exports__, {'default': function() { return __WEBPACK_DEFAULT_EXPORT__; }, 'test': function() { return test; }});
/* harmony import */var _package__WEBPACK_IMPORTED_MODULE_0_ = __webpack_require__(/* ./package */"./package/index.js");

var __WEBPACK_DEFAULT_EXPORT__ = _package__WEBPACK_IMPORTED_MODULE_0_["a"];
 function test() {}
},
"./package/index.js": function (__unused_webpack_module, __webpack_exports__, __webpack_require__) {
'use strict';
__webpack_require__.r(__webpack_exports__);
__webpack_require__.d(__webpack_exports__, {'a': function() { return a; }});

 function a() {
    return 42;
}
 function b() {
    return /* unused */undefined;
}
},

},function(__webpack_require__) {
var __webpack_exec__ = function(moduleId) { return __webpack_require__(__webpack_require__.s = moduleId) }
var __webpack_exports__ = (__webpack_exec__("./index.js"));

}
]);