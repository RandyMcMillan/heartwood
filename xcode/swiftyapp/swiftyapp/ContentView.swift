//
//  ContentView.swift
//  swiftyapp
//
//  Created by Jonathan McKenzie on 7/9/24.
//

import SwiftUI

@MainActor
class HeartwoodStore: ObservableObject {
    @Published var version: String = ""
    @Published var commit: String = ""
    @Published var nodeInfo: HeartwoodNodeInfo?
    @Published var repositories: [HeartwoodRepositoryInfo] = []
    @Published var errorMessage: String?
    @Published var isLoading: Bool = false

    func load() {
        isLoading = true
        defer { isLoading = false }

        version = heartwoodVersion()
        commit = heartwoodCommit()

        do {
            nodeInfo = try heartwoodNodeInfo()
            repositories = try heartwoodRepositoryList()
            errorMessage = nil
        } catch let error as HeartwoodError {
            switch error {
            case .Profile(let msg), .Storage(let msg), .InvalidRepoId(let msg),
                 .InvalidAlias(let msg), .InvalidRelay(let msg), .InvalidNetwork(let msg),
                 .ConfigWrite(let msg), .InvalidAddress(let msg):
                errorMessage = msg
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func dismissError() {
        errorMessage = nil
    }

    func repository(byRid rid: String) -> HeartwoodRepositoryInfo? {
        do {
            return try heartwoodRepository(rid: rid)
        } catch {
            return nil
        }
    }

    func normalizedRepoId(_ input: String) -> String? {
        normalizeRepoId(input: input)
    }

    func normalizedNodeId(_ input: String) -> String? {
        normalizeNodeId(input: input)
    }

    func normalizedAlias(_ input: String) -> String? {
        normalizeAlias(input: input)
    }

    func setAlias(_ value: String) {
        do {
            try heartwoodSetAlias(newAlias: value)
            load()
        } catch let error as HeartwoodError {
            switch error {
            case .InvalidAlias(let msg):
                errorMessage = msg
            default:
                errorMessage = error.localizedDescription
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func setRelay(_ value: String) {
        do {
            try heartwoodSetRelay(mode: value)
            load()
        } catch let error as HeartwoodError {
            switch error {
            case .InvalidRelay(let msg):
                errorMessage = msg
            default:
                errorMessage = error.localizedDescription
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func setNetwork(_ value: String) {
        do {
            try heartwoodSetNetwork(mode: value)
            load()
        } catch let error as HeartwoodError {
            switch error {
            case .InvalidNetwork(let msg):
                errorMessage = msg
            default:
                errorMessage = error.localizedDescription
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func addExternalAddress(_ value: String) {
        do {
            try heartwoodAddExternalAddress(address: value)
            load()
        } catch let error as HeartwoodError {
            switch error {
            case .InvalidAddress(let msg):
                errorMessage = msg
            default:
                errorMessage = error.localizedDescription
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func removeExternalAddress(_ value: String) {
        do {
            try heartwoodRemoveExternalAddress(address: value)
            load()
        } catch let error as HeartwoodError {
            switch error {
            case .InvalidAddress(let msg):
                errorMessage = msg
            default:
                errorMessage = error.localizedDescription
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    var hasProfile: Bool {
        heartwoodHasProfile()
    }
}

struct ContentView: View {
    @Environment(\.colorScheme) private var colorScheme
    @StateObject private var store = HeartwoodStore()
    @State private var newAddress = ""
    @State private var lookupRid = ""
    @State private var lookedUpRepo: HeartwoodRepositoryInfo?

    var body: some View {
        ZStack {
            LinearGradient(
                colors: [
                    colorScheme == .dark
                        ? Color(red: 0.05, green: 0.05, blue: 0.07)
                        : Color(red: 0.95, green: 0.96, blue: 0.98),
                    colorScheme == .dark
                        ? Color(red: 0.10, green: 0.08, blue: 0.06)
                        : Color(red: 0.90, green: 0.92, blue: 0.96),
                    colorScheme == .dark
                        ? Color(red: 0.18, green: 0.09, blue: 0.03)
                        : Color(red: 0.82, green: 0.86, blue: 0.93)
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            if colorScheme == .dark {
                RadialGradient(
                    colors: [
                        Color(red: 1.0, green: 0.60, blue: 0.15).opacity(0.20),
                        .clear
                    ],
                    center: .topTrailing,
                    startRadius: 20,
                    endRadius: 340
                )
                .ignoresSafeArea()
            } else {
                RadialGradient(
                    colors: [
                        Color(red: 0.77, green: 0.86, blue: 1.0).opacity(0.34),
                        .clear
                    ],
                    center: .topTrailing,
                    startRadius: 24,
                    endRadius: 360
                )
                .ignoresSafeArea()
            }

            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    header

                    if let error = store.errorMessage {
                        glassCard {
                            HStack(alignment: .top, spacing: 12) {
                                VStack(alignment: .leading, spacing: 8) {
                                    Label("Error", systemImage: "exclamationmark.triangle.fill")
                                        .font(.headline)
                                        .foregroundStyle(accentText)
                                    Text(error)
                                        .font(.subheadline)
                                        .foregroundStyle(primaryText.opacity(0.84))
                                }
                                Spacer()
                                Button {
                                    store.dismissError()
                                } label: {
                                    Image(systemName: "xmark.circle.fill")
                                        .font(.title3)
                                        .foregroundStyle(accentFill.opacity(0.7))
                                }
                            }
                        }
                    }

                    if !store.hasProfile {
                        glassCard {
                            VStack(alignment: .leading, spacing: 10) {
                                Label("No Profile", systemImage: "person.crop.circle.badge.xmark")
                                    .font(.headline)
                                    .foregroundStyle(accentText)
                                Text("No Heartwood profile was found on this device. Use the radicle CLI to create one before using this app.")
                                    .font(.subheadline)
                                    .foregroundStyle(primaryText.opacity(0.84))
                            }
                        }
                    }

                    if let node = store.nodeInfo {
                        nodeCard(node: node)
                        pathsCard(paths: node.paths)
                    }

                    if !store.repositories.isEmpty {
                        repositoriesCard(repos: store.repositories)
                    }

                    lookupCard

                    bridgeCard
                }
                .padding(20)
            }
            .refreshable {
                store.load()
            }
        }
        .onAppear(perform: store.load)
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Heartwood")
                        .font(.system(size: 34, weight: .bold, design: .rounded))
                        .foregroundStyle(primaryText)

                    Text("SwiftUI talking to Rust via UniFFI.")
                        .font(.subheadline)
                        .foregroundStyle(primaryText.opacity(0.74))
                }

                Spacer()

                Button {
                    store.load()
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .font(.title2)
                        .foregroundStyle(accentFill)
                }
                .disabled(store.isLoading)

                Image(colorScheme == .dark ? "RustOrb" : "RustOrbLight")
                    .resizable()
                    .scaledToFit()
                    .frame(width: 76, height: 76)
                    .padding(12)
                    .background(
                        colorScheme == .dark
                            ? .white.opacity(0.04)
                            : .white.opacity(0.78),
                        in: RoundedRectangle(cornerRadius: 22, style: .continuous)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 22, style: .continuous)
                            .stroke(
                                colorScheme == .dark
                                    ? Color(red: 1.0, green: 0.66, blue: 0.24).opacity(0.26)
                                    : Color(red: 0.52, green: 0.62, blue: 0.72).opacity(0.22),
                                lineWidth: 1
                            )
                    )
            }

            HStack(spacing: 8) {
                pill(text: store.version)
                pill(text: String(store.commit.prefix(7)))
                pill(text: "SwiftUI")
                pill(text: "UniFFI")
                pill(text: "Rust")
            }
        }
        .padding(.bottom, 4)
    }

    @State private var editingAlias = ""

    private func nodeCard(node: HeartwoodNodeInfo) -> some View {
        glassCard {
            VStack(alignment: .leading, spacing: 10) {
                Label("Node", systemImage: "network")
                    .font(.headline)
                    .foregroundStyle(accentText)

                HStack(spacing: 8) {
                    Text("Alias")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(primaryText.opacity(0.62))
                        .frame(width: 72, alignment: .leading)
                    TextField(node.alias, text: $editingAlias)
                        .font(.subheadline)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        store.setAlias(editingAlias)
                        editingAlias = ""
                    } label: {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundStyle(accentFill)
                    }
                    .disabled(editingAlias.isEmpty)
                }

                infoRow(label: "Node ID", value: node.nodeId)
                infoRow(label: "User Agent", value: node.userAgent)

                VStack(alignment: .leading, spacing: 6) {
                    Text("Network")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(primaryText.opacity(0.62))
                    Picker("Network", selection: .init(
                        get: { node.network },
                        set: { store.setNetwork($0) }
                    )) {
                        Text("Main").tag("main")
                        Text("Test").tag("test")
                    }
                    .pickerStyle(.segmented)
                    .tint(accentFill)
                }

                VStack(alignment: .leading, spacing: 6) {
                    Text("Relay")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(primaryText.opacity(0.62))
                    Picker("Relay", selection: .init(
                        get: { node.relay },
                        set: { store.setRelay($0) }
                    )) {
                        Text("Always").tag("always")
                        Text("Never").tag("never")
                        Text("Auto").tag("auto")
                    }
                    .pickerStyle(.segmented)
                    .tint(accentFill)
                }

                if !node.externalAddresses.isEmpty {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("External")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(primaryText.opacity(0.62))
                        ForEach(node.externalAddresses, id: \.self) { addr in
                            HStack {
                                Text(addr)
                                    .font(.subheadline)
                                    .foregroundStyle(primaryText.opacity(0.92))
                                Spacer()
                                Button {
                                    store.removeExternalAddress(addr)
                                } label: {
                                    Image(systemName: "xmark.circle.fill")
                                        .foregroundStyle(accentFill.opacity(0.7))
                                }
                            }
                        }
                    }
                }

                HStack(spacing: 8) {
                    TextField("host:port", text: $newAddress)
                        .font(.subheadline)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        store.addExternalAddress(newAddress)
                        newAddress = ""
                    } label: {
                        Image(systemName: "plus.circle.fill")
                            .foregroundStyle(accentFill)
                    }
                    .disabled(newAddress.isEmpty)
                }

                if !node.connectAddresses.isEmpty {
                    infoRow(label: "Connect", value: node.connectAddresses.joined(separator: ", "))
                }
            }
        }
    }

    private func pathsCard(paths: HeartwoodPaths) -> some View {
        glassCard {
            VStack(alignment: .leading, spacing: 10) {
                Label("Paths", systemImage: "folder")
                    .font(.headline)
                    .foregroundStyle(accentText)

                infoRow(label: "Home", value: paths.home)
                infoRow(label: "Storage", value: paths.storage)
                infoRow(label: "Config", value: paths.config)
                infoRow(label: "Keys", value: paths.keys)
                infoRow(label: "Node", value: paths.node)
            }
        }
    }

    private func repositoriesCard(repos: [HeartwoodRepositoryInfo]) -> some View {
        glassCard {
            VStack(alignment: .leading, spacing: 12) {
                Label("Repositories", systemImage: "archivebox")
                    .font(.headline)
                    .foregroundStyle(accentText)

                ForEach(repos, id: \.rid) { repo in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack {
                            Text(repo.project?.name ?? repo.rid)
                                .font(.subheadline.weight(.semibold))
                                .foregroundStyle(primaryText)
                            Spacer()
                            Text(repo.visibility)
                                .font(.caption.weight(.semibold))
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(accentFill.opacity(colorScheme == .dark ? 0.18 : 0.12), in: Capsule())
                                .foregroundStyle(accentText)
                        }

                        if let project = repo.project {
                            Text(project.description)
                                .font(.caption)
                                .foregroundStyle(primaryText.opacity(0.68))
                                .lineLimit(2)

                            Text("default: \(project.defaultBranch)")
                                .font(.caption2)
                                .foregroundStyle(primaryText.opacity(0.52))
                        }

                        if let head = repo.head {
                            Text("head: \(String(head.prefix(7)))")
                                .font(.caption2.monospaced())
                                .foregroundStyle(primaryText.opacity(0.52))
                        }

                        Text("delegates: \(repo.delegates.count)  threshold: \(repo.threshold)")
                            .font(.caption2)
                            .foregroundStyle(primaryText.opacity(0.52))

                        Text("refs: \(repo.refsState)")
                            .font(.caption2)
                            .foregroundStyle(primaryText.opacity(0.52))
                    }
                    .padding(.vertical, 8)
                    .padding(.horizontal, 12)
                    .background(cardBackground.opacity(colorScheme == .dark ? 0.3 : 0.5), in: RoundedRectangle(cornerRadius: 14, style: .continuous))

                    if repo.rid != repos.last?.rid {
                        Divider()
                            .overlay(accentFill.opacity(colorScheme == .dark ? 0.22 : 0.16))
                    }
                }
            }
        }
    }

    private var lookupCard: some View {
        glassCard {
            VStack(alignment: .leading, spacing: 12) {
                Label("Lookup", systemImage: "magnifyingglass")
                    .font(.headline)
                    .foregroundStyle(accentText)

                HStack(spacing: 8) {
                    TextField("rad:z…", text: $lookupRid)
                        .font(.subheadline)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        lookedUpRepo = store.repository(byRid: lookupRid)
                    } label: {
                        Image(systemName: "magnifyingglass.circle.fill")
                            .foregroundStyle(accentFill)
                    }
                    .disabled(lookupRid.isEmpty)
                }

                if let repo = lookedUpRepo {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack {
                            Text(repo.project?.name ?? repo.rid)
                                .font(.subheadline.weight(.semibold))
                                .foregroundStyle(primaryText)
                            Spacer()
                            Text(repo.visibility)
                                .font(.caption.weight(.semibold))
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(accentFill.opacity(colorScheme == .dark ? 0.18 : 0.12), in: Capsule())
                                .foregroundStyle(accentText)
                        }
                        if let project = repo.project {
                            Text(project.description)
                                .font(.caption)
                                .foregroundStyle(primaryText.opacity(0.68))
                                .lineLimit(2)
                        }
                        if let head = repo.head {
                            Text("head: \(String(head.prefix(7)))")
                                .font(.caption2.monospaced())
                                .foregroundStyle(primaryText.opacity(0.52))
                        }
                    }
                    .padding(.vertical, 8)
                    .padding(.horizontal, 12)
                    .background(cardBackground.opacity(colorScheme == .dark ? 0.3 : 0.5), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                } else if !lookupRid.isEmpty {
                    Text("Repository not found")
                        .font(.subheadline)
                        .foregroundStyle(primaryText.opacity(0.62))
                }
            }
        }
    }

    private var bridgeCard: some View {
        glassCard {
            VStack(alignment: .leading, spacing: 10) {
                Label("Bridge", systemImage: "sparkles")
                    .font(.headline)
                    .foregroundStyle(accentText)

                Text(rustHello())
                    .font(.title2.weight(.semibold))
                    .foregroundStyle(primaryText)

                HStack(spacing: 8) {
                    Text("Answer:")
                        .font(.subheadline)
                        .foregroundStyle(primaryText.opacity(0.74))
                    Text("\(heartwoodAnswer())")
                        .font(.title3.weight(.bold).monospacedDigit())
                        .foregroundStyle(accentText)
                }
            }
        }
    }

    private func infoRow(label: String, value: String) -> some View {
        HStack(alignment: .top, spacing: 8) {
            Text(label)
                .font(.caption.weight(.semibold))
                .foregroundStyle(primaryText.opacity(0.62))
                .frame(width: 72, alignment: .leading)
            Text(value)
                .font(.subheadline)
                .foregroundStyle(primaryText.opacity(0.92))
                .lineLimit(1)
                .truncationMode(.middle)
            Spacer()
        }
    }

    private func pill(text: String) -> some View {
        Text(text)
            .font(.caption.weight(.semibold))
            .padding(.horizontal, 12)
            .padding(.vertical, 7)
            .background(accentFill.opacity(colorScheme == .dark ? 0.14 : 0.10), in: Capsule())
            .foregroundStyle(accentText)
    }

    private func glassCard<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        content()
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(cardBackground, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 24, style: .continuous)
                    .stroke(accentFill.opacity(colorScheme == .dark ? 0.20 : 0.14), lineWidth: 1)
            )
            .shadow(color: .black.opacity(colorScheme == .dark ? 0.28 : 0.12), radius: 18, x: 0, y: 10)
    }

    private var primaryText: Color {
        colorScheme == .dark ? .white : Color(red: 0.10, green: 0.14, blue: 0.20)
    }

    private var accentFill: Color {
        colorScheme == .dark ? Color(red: 1.0, green: 0.66, blue: 0.24) : Color(red: 0.82, green: 0.38, blue: 0.10)
    }

    private var accentText: Color {
        colorScheme == .dark ? Color(red: 1.0, green: 0.86, blue: 0.62) : Color(red: 0.42, green: 0.24, blue: 0.12)
    }

    private var cardBackground: Color {
        colorScheme == .dark ? .white.opacity(0.05) : .white.opacity(0.76)
    }
}

#Preview {
    ContentView()
}
