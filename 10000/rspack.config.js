const path = require("path");
const rspack = require("@rspack/core");
const ReactRefreshPlugin = require("@rspack/plugin-react-refresh");

const prod = process.env.NODE_ENV === "production";
/** @type {import("@rspack/cli").Configuration} */
module.exports = {
	resolve: {
		extensions: [".js", ".jsx"]
	},
	entry: { main: "./index.jsx" },
	plugins: [
		new rspack.HtmlRspackPlugin({
			template: path.resolve(__dirname, "./index.html")
		}),
		!prod && new ReactRefreshPlugin(),
		new rspack.ProgressPlugin()
	].filter(Boolean),

	experiments: {
		cache: {
			type: "persistent",
			storage: {
				type: "filesystem",
				directory: path.resolve(__dirname, "../.cache")
			}
		}
	},
	module: {
		rules: [
			{
				test: /\.(j|t)s$/,
				exclude: [/[\\/]node_modules[\\/]/],
				loader: "builtin:swc-loader",
				options: {
					sourceMap: true,
					jsc: {
						parser: {
							syntax: "typescript"
						},
						externalHelpers: true
					},
					env: {
						targets: "Chrome >= 48"
					}
				}
			},
			{
				test: /\.(j|t)sx$/,
				loader: "builtin:swc-loader",
				exclude: [/[\\/]node_modules[\\/]/],
				options: {
					sourceMap: true,
					jsc: {
						parser: {
							syntax: "typescript",
							tsx: true
						},
						transform: {
							react: {
								runtime: "automatic",
								development: !prod,
								refresh: !prod
							}
						},
						externalHelpers: true
					},
					env: {
						targets: "Chrome >= 48"
					}
				}
			}
		]
	},
	optimization: {
		splitChunks: {
			chunks: "all",
			cacheGroups: {
				d1: {
					test: /\/d1\//
				}
			}
		}
	}
};

module.exports.plugins = module.exports.plugins || [];
module.exports.plugins.push(new (require("../../lib/scenarios/build-plugin.cjs"))());
module.exports.optimization = module.exports.optimization || {}
module.exports.optimization.minimize = false
