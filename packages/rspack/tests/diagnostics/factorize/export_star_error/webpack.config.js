module.exports = {
	builtins: {
		treeShaking: true,
		define: {
			"process.env.NODE_ENV": "development"
		}
	},
	experiments: {
		rspackFuture: {
			newTreeshaking: false
		}
	}
}
