const fs = require('fs');

const file = 'tests/anchor-payroll-capstone-q1-26.ts';
let code = fs.readFileSync(file, 'utf8');

const injection = `
    const reserveInfo = await connection.getAccountInfo(RESERVE);
    let reserveData = reserveInfo!.data;
    reserveData.writeBigUInt64LE(0n, 16); // Set last_update.slot to 0
    await fetch("http://127.0.0.1:8899", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_setAccount",
        params: [
          RESERVE.toBase58(),
          { lamports: reserveInfo!.lamports, data: reserveData.toString("hex"), owner: reserveInfo!.owner.toBase58(), executable: false, rentEpoch: 0 }
        ]
      })
    });
    console.log("✅ Reserve last_update slot hijacked to 0");
`;

if (!code.includes("✅ Reserve last_update slot hijacked")) {
    code = code.replace(
        `// Give Surfpool time to process the account override`,
        `${injection}\n    // Give Surfpool time to process the account override`
    );
    fs.writeFileSync(file, code);
    console.log("Updated!");
}
