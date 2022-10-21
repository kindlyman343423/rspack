const path = require("path");

module.exports = {
	module: {
		rules: [
			{
				test: "\\.s[ac]ss$",
				uses: [{ builtinLoader: "sass-loader" }],
				type: "css"
			}
		]
	},
	resolve: {
		alias: {
			"path-to-alias": path.resolve(__dirname, "a", `alias.scss`),
			"@scss": path.resolve(__dirname, "b", "directory-6", `_index.scss`),
			"@path-to-scss-dir": path.resolve(__dirname, "b"),
			"@/path-to-scss-dir": path.resolve(__dirname, "b")
		}
	}
};
