/** @type {import('next').NextConfig} */
const nextConfig = {
    output: "export",
    transpilePackages: ["react-leaflet-cluster"],
    experimental: { esmExternals: 'loose' }
};

export default nextConfig;
